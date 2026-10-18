//! Import infrastructure definitions as diagrams: docker-compose, Kubernetes YAML, Terraform.
//! Each importer emits idraw code, which `code::apply` turns into a laid-out diagram.

use crate::code::{guess_icon, slug};
use crate::model::Doc;
use yaml_rust2::{Yaml, YamlLoader};

/// Detects the format and returns idraw code.
pub fn to_code(src: &str, file_name: &str) -> Result<String, String> {
    if let Some(c) = crate::code::from_mermaid(src) {
        return Ok(c);
    }
    let lower = file_name.to_lowercase();
    if lower.ends_with(".tf") || src.contains("resource \"") {
        return terraform(src);
    }
    let docs = YamlLoader::load_from_str(src).map_err(|e| format!("YAML: {e}"))?;
    if docs.iter().any(|d| !d["services"].is_badvalue()) {
        return compose(&docs[0]);
    }
    if docs.iter().any(|d| d["kind"].as_str().is_some()) {
        return kubernetes(&docs);
    }
    Err("Unrecognized file. Supported: docker-compose, Kubernetes YAML, Terraform (.tf), Mermaid".into())
}

/// Builds a fresh, laid-out doc from source.
pub fn to_doc(src: &str, file_name: &str) -> Result<Doc, String> {
    let code = to_code(src, file_name)?;
    let mut d = Doc::default();
    crate::code::apply(&mut d, &code)?;
    Ok(d)
}

fn def(key: &str, icon: &str, title: &str, sub: &str) -> String {
    format!("{key} = {icon} \"{}\" \"{}\"\n", title.replace('"', "'"), sub.replace('"', "'"))
}

fn image_icon(name: &str, image: &str) -> &'static str {
    let g = guess_icon(&format!("{image} {name}"));
    if g == "service" { "container" } else { g }
}

fn compose(doc: &Yaml) -> Result<String, String> {
    let services = doc["services"].as_hash().ok_or("`services` is not a map")?;
    let (mut defs, mut edges) = (String::new(), String::new());
    for (k, v) in services {
        let name = k.as_str().unwrap_or("service");
        let image = v["image"].as_str().unwrap_or(if v["build"].is_badvalue() { "" } else { "local build" });
        defs += &def(&slug(name), image_icon(name, image), name, image);
        let deps: Vec<String> = match &v["depends_on"] {
            Yaml::Array(a) => a.iter().filter_map(|d| d.as_str().map(String::from)).collect(),
            Yaml::Hash(h) => h.keys().filter_map(|d| d.as_str().map(String::from)).collect(),
            _ => vec![],
        };
        let links: Vec<String> = v["links"].as_vec().map(|a| a.iter().filter_map(|d| d.as_str().map(|s| s.split(':').next().unwrap().to_string())).collect()).unwrap_or_default();
        for d in deps.iter().chain(&links) {
            edges += &format!("{} -> {}\n", slug(name), slug(d));
        }
    }
    Ok(format!("{defs}\n{edges}"))
}

fn kubernetes(docs: &[Yaml]) -> Result<String, String> {
    let (mut defs, mut edges) = (String::new(), String::new());
    struct Workload {
        key: String,
        labels: Vec<(String, String)>,
    }
    let mut workloads = vec![];
    let mut services = vec![];
    let items: Vec<&Yaml> = docs.iter().flat_map(|d| if d["kind"].as_str() == Some("List") { d["items"].as_vec().map(|v| v.iter().collect()).unwrap_or_default() } else { vec![d] }).collect();
    for d in items {
        let kind = d["kind"].as_str().unwrap_or("");
        let name = d["metadata"]["name"].as_str().unwrap_or("unnamed");
        match kind {
            "Deployment" | "StatefulSet" | "DaemonSet" | "Job" | "CronJob" => {
                let spec = if kind == "CronJob" { &d["spec"]["jobTemplate"]["spec"]["template"] } else { &d["spec"]["template"] };
                let image = spec["spec"]["containers"][0]["image"].as_str().unwrap_or("");
                let icon = if kind == "CronJob" { "clock" } else if kind == "StatefulSet" && guess_icon(image) == "service" { "database" } else { image_icon(name, image) };
                let key = slug(name);
                let replicas = d["spec"]["replicas"].as_i64().map(|r| format!(" · ×{r}")).unwrap_or_default();
                defs += &def(&key, icon, name, &format!("{kind}{replicas}"));
                let labels = spec["metadata"]["labels"].as_hash().map(|h| h.iter().filter_map(|(k, v)| Some((k.as_str()?.to_string(), v.as_str()?.to_string()))).collect()).unwrap_or_default();
                workloads.push(Workload { key, labels });
            }
            "Service" => {
                let key = format!("{}_svc", slug(name));
                defs += &def(&key, "lb", name, &format!("Service · {}", d["spec"]["type"].as_str().unwrap_or("ClusterIP")));
                let sel: Vec<(String, String)> = d["spec"]["selector"].as_hash().map(|h| h.iter().filter_map(|(k, v)| Some((k.as_str()?.to_string(), v.as_str()?.to_string()))).collect()).unwrap_or_default();
                services.push((name.to_string(), key, sel));
            }
            "Ingress" => {
                let key = format!("{}_ingress", slug(name));
                defs += &def(&key, "gateway", name, "Ingress");
                for rule in d["spec"]["rules"].as_vec().into_iter().flatten() {
                    for p in rule["http"]["paths"].as_vec().into_iter().flatten() {
                        let svc = p["backend"]["service"]["name"].as_str().or(p["backend"]["serviceName"].as_str());
                        if let Some(svc) = svc {
                            edges += &format!("{key} -> {}_svc : {}\n", slug(svc), p["path"].as_str().unwrap_or("/"));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    for (_, key, sel) in &services {
        for w in &workloads {
            if !sel.is_empty() && sel.iter().all(|s| w.labels.contains(s)) {
                edges += &format!("{key} -> {}\n", w.key);
            }
        }
    }
    if defs.is_empty() {
        return Err("No Deployments, Services or Ingresses found".into());
    }
    Ok(format!("{defs}\n{edges}"))
}

/// Maps a Terraform resource type to an icon; None = infrastructure noise we skip.
fn tf_icon(t: &str) -> Option<&'static str> {
    const NOISE: &[&str] = &["iam_", "security_group", "route_table", "route", "subnet", "internet_gateway", "policy", "attachment", "association", "_rule", "permission", "random_", "null_", "tls_", "local_", "_version", "eip", "key_pair", "launch_template", "target_group", "listener", "parameter_group", "option_group", "log_group"];
    if NOISE.iter().any(|n| t.contains(n)) {
        return None;
    }
    let map: &[(&str, &str)] = &[
        ("aws_instance", "aws-ec2"),
        ("aws_lambda", "aws-lambda"),
        ("aws_s3", "aws-s3"),
        ("aws_db_instance", "aws-rds"),
        ("aws_rds", "aws-aurora"),
        ("aws_dynamodb", "aws-dynamodb"),
        ("aws_sqs", "aws-sqs"),
        ("aws_sns", "aws-sns"),
        ("aws_lb", "aws-elb"),
        ("aws_alb", "aws-elb"),
        ("aws_elb", "aws-elb"),
        ("aws_api_gateway", "aws-apigw"),
        ("aws_apigatewayv2", "aws-apigw"),
        ("aws_cloudfront", "aws-cloudfront"),
        ("aws_route53", "aws-route53"),
        ("aws_vpc", "aws-vpc"),
        ("aws_nat_gateway", "aws-natgw"),
        ("aws_ecs", "aws-ecs"),
        ("aws_eks", "aws-eks"),
        ("aws_elasticache", "aws-elasticache"),
        ("aws_kinesis", "aws-kinesis"),
        ("aws_msk", "aws-msk"),
        ("aws_cognito", "aws-cognito"),
        ("aws_kms", "aws-kms"),
        ("aws_secretsmanager", "aws-secrets"),
        ("aws_cloudwatch", "aws-cloudwatch"),
        ("aws_sfn", "aws-stepfunctions"),
        ("aws_ecr", "aws-ecr"),
        ("aws_opensearch", "aws-opensearch"),
        ("aws_elasticsearch", "aws-opensearch"),
        ("aws_redshift", "aws-redshift"),
        ("aws_efs", "aws-efs"),
        ("aws_wafv2", "aws-waf"),
        ("aws_cloudwatch_event", "aws-eventbridge"),
        ("google_compute_instance", "gcp-gce"),
        ("google_cloud_run", "gcp-run"),
        ("google_cloudfunctions", "gcp-functions"),
        ("google_container_cluster", "gcp-gke"),
        ("google_storage_bucket", "gcp-gcs"),
        ("google_sql", "gcp-sql"),
        ("google_spanner", "gcp-spanner"),
        ("google_bigquery", "gcp-bigquery"),
        ("google_pubsub", "gcp-pubsub"),
        ("google_redis", "gcp-memorystore"),
        ("google_compute_network", "gcp-vpc"),
        ("google_firestore", "gcp-firestore"),
        ("google_bigtable", "gcp-bigtable"),
        ("google_compute_backend_service", "gcp-lb"),
        ("google_compute_url_map", "gcp-lb"),
        ("google_dns", "gcp-dns"),
        ("google_secret_manager", "gcp-secret"),
        ("google_kms", "gcp-kms"),
        ("azurerm_linux_virtual_machine", "az-vm"),
        ("azurerm_windows_virtual_machine", "az-vm"),
        ("azurerm_virtual_machine", "az-vm"),
        ("azurerm_linux_web_app", "az-appservice"),
        ("azurerm_app_service", "az-appservice"),
        ("azurerm_function_app", "az-functions"),
        ("azurerm_linux_function_app", "az-functions"),
        ("azurerm_kubernetes_cluster", "az-aks"),
        ("azurerm_container_app", "az-containerapps"),
        ("azurerm_storage_account", "az-blob"),
        ("azurerm_mssql", "az-sql"),
        ("azurerm_postgresql", "az-postgres"),
        ("azurerm_cosmosdb", "az-cosmos"),
        ("azurerm_redis_cache", "az-redis"),
        ("azurerm_eventhub", "az-eventhubs"),
        ("azurerm_servicebus", "az-servicebus"),
        ("azurerm_eventgrid", "az-eventgrid"),
        ("azurerm_api_management", "az-apim"),
        ("azurerm_cdn_frontdoor", "az-frontdoor"),
        ("azurerm_frontdoor", "az-frontdoor"),
        ("azurerm_virtual_network", "az-vnet"),
        ("azurerm_key_vault", "az-keyvault"),
        ("azurerm_application_gateway", "az-appgw"),
        ("azurerm_lb", "az-lb"),
        ("azurerm_dns_zone", "az-dns"),
        ("azurerm_log_analytics", "az-loganalytics"),
        ("azurerm_application_insights", "az-appinsights"),
        ("azurerm_container_registry", "az-acr"),
        ("cloudflare_worker", "cf-workers"),
        ("cloudflare_r2", "cf-r2"),
        ("cloudflare_record", "cf-dns"),
        ("cloudflare_d1", "cf-d1"),
    ];
    // longest matching prefix wins
    map.iter().filter(|(p, _)| t.starts_with(p)).max_by_key(|(p, _)| p.len()).map(|(_, i)| *i).or(Some(guess_icon(t)))
}

fn terraform(src: &str) -> Result<String, String> {
    // ponytail: brace-matching scanner, not a full HCL parser; good for resource graphs.
    struct Res {
        ty: String,
        name: String,
        body: String,
        icon: &'static str,
    }
    let mut res: Vec<Res> = vec![];
    let mut rest = src;
    while let Some(i) = rest.find("resource \"") {
        let after = &rest[i + 10..];
        let mut q = after.split('"');
        let ty = q.next().unwrap_or("").to_string();
        let _ = q.next();
        let name = q.next().unwrap_or("").to_string();
        let Some(open) = after.find('{') else { break };
        let mut depth = 0;
        let mut end = after.len();
        for (j, c) in after[open..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = open + j;
                        break;
                    }
                }
                _ => {}
            }
        }
        let body = after[open..end].to_string();
        rest = &after[end..];
        if let Some(icon) = tf_icon(&ty) {
            res.push(Res { ty, name, body, icon });
        }
    }
    if res.is_empty() {
        return Err("No diagrammable Terraform resources found".into());
    }
    let key = |r: &Res| slug(&format!("{}_{}", r.ty.split('_').skip(1).collect::<Vec<_>>().join("_"), r.name));
    let (mut defs, mut edges) = (String::new(), String::new());
    for r in &res {
        let title = crate::icons::find(r.icon).map(|i| i.label).unwrap_or(&r.ty);
        defs += &def(&key(r), r.icon, title, &r.name);
        for o in &res {
            if std::ptr::eq(o, r) {
                continue;
            }
            if r.body.contains(&format!("{}.{}", o.ty, o.name)) {
                edges += &format!("{} -> {}\n", key(r), key(o));
            }
        }
    }
    Ok(format!("{defs}\n{edges}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Kind;

    fn nodes(d: &Doc) -> usize {
        d.elements.iter().filter(|e| e.kind == Kind::Node).count()
    }
    fn arrows(d: &Doc) -> usize {
        d.elements.iter().filter(|e| e.kind == Kind::Arrow).count()
    }

    #[test]
    fn compose_file() {
        let y = "services:\n  web:\n    image: nginx\n    depends_on: [api]\n  api:\n    build: .\n    depends_on:\n      db:\n        condition: service_healthy\n  db:\n    image: postgres:16\n";
        let d = to_doc(y, "docker-compose.yml").unwrap();
        assert_eq!((nodes(&d), arrows(&d)), (3, 2));
        assert!(d.elements.iter().any(|e| e.icon == "database"));
    }

    #[test]
    fn k8s_manifests() {
        let y = "apiVersion: apps/v1\nkind: Deployment\nmetadata: {name: api}\nspec:\n  replicas: 3\n  template:\n    metadata: {labels: {app: api}}\n    spec: {containers: [{name: api, image: ghcr.io/x/api}]}\n---\nkind: Service\nmetadata: {name: api}\nspec: {selector: {app: api}}\n---\nkind: Ingress\nmetadata: {name: web}\nspec:\n  rules:\n  - http:\n      paths:\n      - path: /\n        backend: {service: {name: api}}\n";
        let d = to_doc(y, "k8s.yaml").unwrap();
        assert_eq!((nodes(&d), arrows(&d)), (3, 2));
    }

    #[test]
    fn terraform_resources() {
        let tf = r#"
resource "aws_lambda_function" "handler" { role = aws_iam_role.r.arn  environment { variables = { TABLE = aws_dynamodb_table.orders.name } } }
resource "aws_dynamodb_table" "orders" { name = "orders" }
resource "aws_iam_role" "r" { name = "x" }
resource "aws_api_gateway_rest_api" "api" { body = aws_lambda_function.handler.invoke_arn }
"#;
        let d = to_doc(tf, "main.tf").unwrap();
        assert_eq!((nodes(&d), arrows(&d)), (3, 2));
        assert!(d.elements.iter().any(|e| e.icon == "aws-dynamodb"));
    }
}
