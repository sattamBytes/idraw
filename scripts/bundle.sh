#!/usr/bin/env bash
# Builds dist/idraw.app (release binary + icon).
#   scripts/bundle.sh              build for this Mac
#   scripts/bundle.sh --install    ...and copy to ~/Applications (Spotlight / Launchpad)
#   scripts/bundle.sh --universal  Apple Silicon + Intel binary, plus a release zip in dist/
set -euo pipefail
cd "$(dirname "$0")/.."

INSTALL=0
UNIVERSAL=0
for a in "$@"; do
  case "$a" in
    --install) INSTALL=1 ;;
    --universal) UNIVERSAL=1 ;;
    *) echo "unknown option: $a" >&2; exit 1 ;;
  esac
done

VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
APP=dist/idraw.app
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

if [[ $UNIVERSAL == 1 ]]; then
  rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null
  cargo build --release --target aarch64-apple-darwin
  cargo build --release --target x86_64-apple-darwin
  lipo -create -output "$APP/Contents/MacOS/idraw" \
    target/aarch64-apple-darwin/release/idraw target/x86_64-apple-darwin/release/idraw
else
  cargo build --release
  cp target/release/idraw "$APP/Contents/MacOS/idraw"
fi

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
echo "Built $APP ($(lipo -archs "$APP/Contents/MacOS/idraw"))"

if [[ $UNIVERSAL == 1 ]]; then
  ZIP="dist/idraw-v${VERSION}-macos-universal.zip"
  rm -f "$ZIP"
  ditto -c -k --keepParent "$APP" "$ZIP"
  echo "Zipped $ZIP"
  shasum -a 256 "$ZIP"
fi

if [[ $INSTALL == 1 ]]; then
  mkdir -p "$HOME/Applications"
  rm -rf "$HOME/Applications/idraw.app"
  cp -R "$APP" "$HOME/Applications/"
  echo "Installed to ~/Applications/idraw.app"
  # command line (idraw render / check / icons) for scripts and AI agents
  mkdir -p "$HOME/.local/bin"
  ln -sf "$HOME/Applications/idraw.app/Contents/MacOS/idraw" "$HOME/.local/bin/idraw"
  echo "CLI linked at ~/.local/bin/idraw"
fi
