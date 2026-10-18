//! Layered ("Sugiyama-lite") auto-layout: rank by longest path, order by barycenter sweeps.

use crate::model::*;
use eframe::egui::{Rect, Vec2, vec2};
use std::collections::HashMap;

const GAP_MAIN: f32 = 120.0;
const GAP_CROSS: f32 = 44.0;

/// Target rects for laid-out shapes plus refitted regions (and loose items inside them).
pub fn layered(doc: &Doc, scope: &[u64], vertical: bool) -> Vec<(u64, Rect)> {
    let mut nodes: Vec<&Element> = doc
        .elements
        .iter()
        .filter(|e| e.bindable() && !matches!(e.kind, Kind::Text | Kind::Sticky) && (scope.is_empty() || scope.contains(&e.id)))
        .collect();
    if nodes.len() < 2 {
        return vec![];
    }
    nodes.sort_by(|a, b| (a.rect.min.x, a.rect.min.y).partial_cmp(&(b.rect.min.x, b.rect.min.y)).unwrap());
    let n = nodes.len();
    let idx: HashMap<u64, usize> = nodes.iter().enumerate().map(|(i, e)| (e.id, i)).collect();
    let mut adj = vec![vec![]; n];
    for e in doc.elements.iter().filter(|e| e.kind == Kind::Arrow) {
        if let (Some(&a), Some(&b)) = (e.from.id.and_then(|i| idx.get(&i)), e.to.id.and_then(|i| idx.get(&i))) {
            if a != b && !adj[a].contains(&b) {
                adj[a].push(b);
            }
        }
    }
    // Break cycles: DFS, drop back edges.
    let mut state = vec![0u8; n];
    let mut dag = vec![vec![]; n];
    for s in 0..n {
        if state[s] != 0 {
            continue;
        }
        let mut stack = vec![(s, 0usize)];
        state[s] = 1;
        while let Some((v, i)) = stack.pop() {
            if i < adj[v].len() {
                stack.push((v, i + 1));
                let w = adj[v][i];
                match state[w] {
                    0 => {
                        dag[v].push(w);
                        state[w] = 1;
                        stack.push((w, 0));
                    }
                    2 => dag[v].push(w),
                    _ => {} // back edge
                }
            } else {
                state[v] = 2;
            }
        }
    }
    // Longest-path ranks via Kahn.
    let mut indeg = vec![0; n];
    for v in 0..n {
        for &w in &dag[v] {
            indeg[w] += 1;
        }
    }
    let mut rank = vec![0usize; n];
    let mut q: Vec<usize> = (0..n).filter(|&v| indeg[v] == 0).collect();
    while let Some(v) = q.pop() {
        for &w in &dag[v] {
            rank[w] = rank[w].max(rank[v] + 1);
            indeg[w] -= 1;
            if indeg[w] == 0 {
                q.push(w);
            }
        }
    }
    let layers_n = rank.iter().max().unwrap() + 1;
    let mut layers: Vec<Vec<usize>> = vec![vec![]; layers_n];
    for v in 0..n {
        layers[rank[v]].push(v);
    }
    let cross = |e: &Element| if vertical { e.rect.center().x } else { e.rect.center().y };
    for l in &mut layers {
        l.sort_by(|a, b| cross(nodes[*a]).partial_cmp(&cross(nodes[*b])).unwrap());
    }
    let preds: Vec<Vec<usize>> = (0..n).map(|w| (0..n).filter(|v| dag[*v].contains(&w)).collect()).collect();
    let mut pos = vec![0.0f32; n];
    let set_pos = |layers: &Vec<Vec<usize>>, pos: &mut Vec<f32>| {
        for l in layers {
            for (i, v) in l.iter().enumerate() {
                pos[*v] = i as f32;
            }
        }
    };
    set_pos(&layers, &mut pos);
    for sweep in 0..6 {
        let down = sweep % 2 == 0;
        let order: Vec<usize> = if down { (1..layers_n).collect() } else { (0..layers_n.saturating_sub(1)).rev().collect() };
        for li in order {
            let mut keyed: Vec<(f32, usize)> = layers[li]
                .iter()
                .map(|&v| {
                    let nb: Vec<usize> = if down { preds[v].clone() } else { dag[v].clone() };
                    let k = if nb.is_empty() { pos[v] } else { nb.iter().map(|u| pos[*u]).sum::<f32>() / nb.len() as f32 };
                    (k, v)
                })
                .collect();
            keyed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            layers[li] = keyed.into_iter().map(|k| k.1).collect();
            set_pos(&layers, &mut pos);
        }
    }
    // Coordinates.
    let main = |s: Vec2| if vertical { s.y } else { s.x };
    let crs = |s: Vec2| if vertical { s.x } else { s.y };
    let mut out_rect: HashMap<u64, Rect> = HashMap::new();
    let mut m = 0.0;
    for l in &layers {
        let thick = l.iter().map(|v| main(nodes[*v].rect.size())).fold(0.0, f32::max);
        let total: f32 = l.iter().map(|v| crs(nodes[*v].rect.size())).sum::<f32>() + GAP_CROSS * (l.len() as f32 - 1.0);
        let mut c = -total / 2.0;
        for v in l {
            let s = nodes[*v].rect.size();
            let center = if vertical { vec2(c + s.x / 2.0, m + thick / 2.0) } else { vec2(m + thick / 2.0, c + s.y / 2.0) };
            out_rect.insert(nodes[*v].id, Rect::from_center_size(center.to_pos2(), s));
            c += crs(s) + GAP_CROSS;
        }
        m += thick + GAP_MAIN;
    }
    // Keep the drawing where it was.
    let old = nodes.iter().map(|e| e.rect).reduce(|a, b| a.union(b)).unwrap();
    let new = out_rect.values().copied().reduce(|a, b| a.union(b)).unwrap();
    let d = old.center() - new.center();
    for r in out_rect.values_mut() {
        *r = r.translate(d);
    }
    // Refit regions around their members, innermost first.
    let mut regions: Vec<&Element> = doc.elements.iter().filter(|e| e.kind == Kind::Region).collect();
    regions.sort_by(|a, b| (a.rect.area()).partial_cmp(&b.rect.area()).unwrap());
    let mut result: Vec<(u64, Rect)> = out_rect.iter().map(|(k, v)| (*k, *v)).collect();
    let mut moved: HashMap<u64, Rect> = out_rect.clone();
    for reg in regions {
        let members: Vec<u64> =
            doc.elements.iter().filter(|e| e.id != reg.id && e.kind != Kind::Arrow && e.kind != Kind::Pen && reg.rect.contains(e.rect.center()) && reg.rect.area() > e.rect.area()).map(|e| e.id).collect();
        let fitted: Vec<Rect> = members.iter().filter_map(|id| moved.get(id).copied()).collect();
        let Some(b) = fitted.into_iter().reduce(|a, b| a.union(b)) else { continue };
        let nr = Rect::from_min_max(b.min - vec2(32.0, 52.0), b.max + vec2(32.0, 32.0));
        let delta = nr.min - reg.rect.min;
        // loose items (text, stickies) ride along with their region
        for id in &members {
            if !moved.contains_key(id) {
                if let Some(e) = doc.get(*id) {
                    let r = e.rect.translate(delta);
                    moved.insert(*id, r);
                    result.push((*id, r));
                }
            }
        }
        moved.insert(reg.id, nr);
        result.push((reg.id, nr));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::pos2;

    #[test]
    fn chain_goes_left_to_right() {
        let mut d = Doc::default();
        let ids: Vec<u64> = [(0., 300.), (500., 0.), (100., 100.)].iter().map(|(x, y)| d.add(Element::node("service", pos2(*x, *y)))).collect();
        for w in ids.windows(2) {
            let mut a = Element::new(Kind::Arrow, Rect::ZERO);
            a.from.id = Some(w[0]);
            a.to.id = Some(w[1]);
            d.add(a);
        }
        let out: HashMap<u64, Rect> = layered(&d, &[], false).into_iter().collect();
        let x = |i: usize| out[&ids[i]].center().x;
        assert!(x(0) < x(1) && x(1) < x(2));
        let y = |i: usize| out[&ids[i]].center().y;
        assert!((y(0) - y(2)).abs() < 1.0, "chain should be aligned");
    }
}
