#!/bin/bash
# Builds EliteNexus-Gen2 for this Mac (Apple Silicon or Intel) and packs it as "EliteNexus-Gen2.app".
#
#   tools/macos/build-app.sh             build target/macos/EliteNexus-Gen2.app and a zip of it
#   tools/macos/build-app.sh --install   ... and copy the app to /Applications (~/Applications if that isn't writable)
#
# Needs Xcode's Command Line Tools (xcode-select --install) and Rust (https://rustup.rs). Honours CARGO_TARGET_DIR.
# The app keeps its data in ~/Library/Application Support/EliteNexus-Gen2, outside the bundle, so replacing the app
# keeps your settings, galaxy database and upload queues.
set -euo pipefail

install=0
for a in "$@"; do
  case "$a" in
    --install) install=1 ;;
    *) echo "Unknown option: $a (use --install)" >&2; exit 2 ;;
  esac
done

if [ "$(uname -s)" != Darwin ]; then
  echo "This script makes the macOS app and runs on a Mac." >&2
  exit 1
fi
for tool in cargo codesign iconutil ditto plutil; do
  command -v "$tool" >/dev/null || { echo "Missing $tool: install Xcode's Command Line Tools (xcode-select --install) and Rust (https://rustup.rs)." >&2; exit 1; }
done

root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
version="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)"
arch="$(uname -m)"   # arm64 or x86_64
target_dir="${CARGO_TARGET_DIR:-$root/target}"
out="$target_dir/macos"
app="$out/EliteNexus-Gen2.app"

echo "==> Building EliteNexus-Gen2 $version for macOS ($arch)"
cargo +1.100.0 build --locked --release -p nexus-app

echo "==> Packing $app"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$target_dir/release/EliteNexus-Gen2" "$app/Contents/MacOS/EliteNexus-Gen2"

# The icon is drawn in code (crates/nexus-app/src/icon.rs); iconutil turns the rendered sizes into an .icns.
iconset="$out/EliteNexus-Gen2.iconset"
rm -rf "$iconset"
cargo +1.100.0 run --locked --quiet --release -p nexus-app --example macos_iconset -- "$iconset" >/dev/null
iconutil -c icns -o "$app/Contents/Resources/EliteNexus-Gen2.icns" "$iconset"
rm -rf "$iconset"

cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key><string>en</string>
  <key>CFBundleDisplayName</key><string>EliteNexus-Gen2</string>
  <key>CFBundleName</key><string>EliteNexus-Gen2</string>
  <key>CFBundleExecutable</key><string>EliteNexus-Gen2</string>
  <key>CFBundleIconFile</key><string>EliteNexus-Gen2</string>
  <key>CFBundleIdentifier</key><string>io.github.shadowgar.elitenexus-gen2</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.games</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSSupportsAutomaticGraphicsSwitching</key><true/>
  <key>NSHumanReadableCopyright</key><string>Based on Elite Nexus by CMDR Alka Uruskan and contributors. Free software under the GNU GPL, version 3 or later. Elite Dangerous is a trademark of Frontier Developments plc.</string>
</dict>
</plist>
PLIST
plutil -lint -s "$app/Contents/Info.plist"

# The licences travel with the program (GPL for EliteNexus-Gen2, SIL OFL for the bundled Inter font).
cp LICENSE NOTICE.md README.md "$app/Contents/Resources/"
cp docs/USER_GUIDE.md "$app/Contents/Resources/USER_GUIDE.md"
cp data/fonts/inter/LICENSE.txt "$app/Contents/Resources/Inter-LICENSE.txt"

# An ad-hoc signature (no Apple developer account): enough for a Mac to run an app built on it.
codesign --force --sign - --timestamp=none "$app"
codesign --verify --strict "$app"

zip="$out/EliteNexus-Gen2-v$version-macos-$arch.zip"
rm -f "$zip"
ditto -c -k --sequesterRsrc --keepParent "$app" "$zip"
echo "==> Built $app"
echo "    and $zip"

if [ "$install" = 1 ]; then
  dest=/Applications
  if [ ! -w "$dest" ]; then
    dest="$HOME/Applications"
    mkdir -p "$dest"
  fi
  if pgrep -xq EliteNexus-Gen2; then
    echo "    EliteNexus-Gen2 is running: quit it, then run this again to install." >&2
    exit 3
  fi
  rm -rf "$dest/EliteNexus-Gen2.app"
  ditto "$app" "$dest/EliteNexus-Gen2.app"
  echo "==> Installed $dest/EliteNexus-Gen2.app"
fi
