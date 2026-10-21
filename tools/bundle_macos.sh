#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
profile="${1:-release}"
case "$profile" in
  release) cargo build --release ;;
  debug) cargo build ;;
  *) echo 'Usage: tools/bundle_macos.sh [release|debug]' >&2; exit 2 ;;
esac
app="dist/GRID.app/Contents"
mkdir -p "$app/MacOS" "$app/Resources"
cp "target/$profile/car-game" "$app/MacOS/GRID"
cp -R assets "$app/Resources/"
cat > "$app/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleExecutable</key><string>GRID</string>
  <key>CFBundleIdentifier</key><string>local.car-game.grid</string>
  <key>CFBundleName</key><string>GRID</string>
  <key>CFBundleDisplayName</key><string>GRID — Free Drive</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>LSMinimumSystemVersion</key><string>14.0</string>
</dict></plist>
PLIST
echo "Built $(pwd)/dist/GRID.app"
