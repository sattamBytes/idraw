#!/usr/bin/env bash
# Builds dist/idraw.app (release binary + icon). Usage: scripts/bundle.sh [--install]
#   --install  copies it to ~/Applications so it shows up in Spotlight / Launchpad.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo build --release

APP=dist/idraw.app
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/idraw "$APP/Contents/MacOS/idraw"

# icon: SVG -> transparent 1024px PNG (resvg) -> iconset -> icns
TMP=$(mktemp -d)
cargo run --release --quiet --example render_icon -- "$TMP/icon.svg.png"
SET="$TMP/idraw.iconset"
mkdir -p "$SET"
for s in 16 32 128 256 512; do
  sips -z $s $s "$TMP/icon.svg.png" --out "$SET/icon_${s}x${s}.png" >/dev/null
  sips -z $((s * 2)) $((s * 2)) "$TMP/icon.svg.png" --out "$SET/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$SET" -o "$APP/Contents/Resources/idraw.icns"
rm -rf "$TMP"

VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
cat > "$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>idraw</string>
  <key>CFBundleDisplayName</key><string>idraw</string>
  <key>CFBundleIdentifier</key><string>dev.idraw.app</string>
  <key>CFBundleExecutable</key><string>idraw</string>
  <key>CFBundleIconFile</key><string>idraw</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundleVersion</key><string>${VERSION}</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>LSApplicationCategoryType</key><string>public.app-category.graphics-design</string>
</dict>
</plist>
EOF

# ad-hoc sign so Gatekeeper lets a locally built app run
codesign --force --deep --sign - "$APP" >/dev/null 2>&1 || true
echo "Built $APP"

if [[ "${1:-}" == "--install" ]]; then
  mkdir -p "$HOME/Applications"
  rm -rf "$HOME/Applications/idraw.app"
  cp -R "$APP" "$HOME/Applications/"
  echo "Installed to ~/Applications/idraw.app"
fi
