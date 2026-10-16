#!/bin/bash
# Wrap the sam GUI binary in Sam.app so Finder, Spotlight and `open` launch the
# window directly instead of running it inside Terminal.
#
#   scripts/make-app.sh <path/to/sam> <output dir> [version]

# macOS /bin/bash is 3.2. Keep this script compatible with that.

set -eu
set -o pipefail

if [[ $# -lt 2 ]]; then
  echo "usage: $0 <path/to/sam> <output dir> [version]" >&2
  exit 1
fi

binary="$1"
out_dir="$2"
root="$(cd "$(dirname "$0")/.." && pwd)"
version="${3:-$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$root/Cargo.toml" | head -n 1)}"
icon_png="$root/assets/icon_256.png"

[[ -f "$binary" ]] || { echo "$binary not found" >&2; exit 1; }

app="$out_dir/Sam.app"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"

ditto "$binary" "$app/Contents/MacOS/sam"
chmod 755 "$app/Contents/MacOS/sam"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
iconset="$work/Sam.iconset"
mkdir "$iconset"
for size in 16 32 128 256; do
  sips -z "$size" "$size" "$icon_png" --out "$iconset/icon_${size}x${size}.png" >/dev/null
done
for size in 16 32 128; do
  double=$((size * 2))
  sips -z "$double" "$double" "$icon_png" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/Sam.icns"

cat >"$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleDisplayName</key>
  <string>Sam</string>
  <key>CFBundleExecutable</key>
  <string>sam</string>
  <key>CFBundleIconFile</key>
  <string>Sam</string>
  <key>CFBundleIdentifier</key>
  <string>io.github.luisitoluis.sam</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>Sam</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>$version</string>
  <key>CFBundleVersion</key>
  <string>$version</string>
  <key>LSApplicationCategoryType</key>
  <string>public.app-category.utilities</string>
  <key>LSMinimumSystemVersion</key>
  <string>11.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
EOF

# codesign rejects the Finder icon xattrs that rustc-with-icon.sh puts on the binary.
xattr -cr "$app"
codesign --force --deep --sign - "$app" >/dev/null 2>&1 || echo "warning: ad-hoc signing failed" >&2

echo "$app"
