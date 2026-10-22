#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
profile="${1:-release}"
case "$profile" in
  release) python3 tools/record_build.py --profile release ;;
  debug) python3 tools/record_build.py ;;
  *) echo 'Usage: tools/bundle_macos.sh [release|debug]' >&2; exit 2 ;;
esac
app="dist/GRID.app/Contents"
mkdir -p "$app/MacOS" "$app/Resources"
target_dir="${CARGO_TARGET_DIR:-target}"
cp "$target_dir/$profile/car-game.recorded" "$app/MacOS/GRID"
rm -rf "$app/MacOS/GRID.provenance"
cp -R "$target_dir/$profile/car-game.recorded.provenance" "$app/MacOS/GRID.provenance"
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
