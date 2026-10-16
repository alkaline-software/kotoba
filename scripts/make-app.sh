#!/bin/zsh
# Build Kotoba.app: release binary + kotoba-core CLI beside it, Info.plist,
# ad-hoc signature. Usage: scripts/make-app.sh [--run]
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"

APP=dist/Kotoba.app
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)

cargo build --release -p kotoba-app -p kotoba-core --bin Kotoba --bin kotoba-core

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/Kotoba "$APP/Contents/MacOS/Kotoba"
cp target/release/kotoba-core "$APP/Contents/MacOS/kotoba-core"
cp crates/kotoba-app/assets/icon/icon.icns "$APP/Contents/Resources/Kotoba.icns"
cp -R crates/kotoba-app/macos/locales/. "$APP/Contents/Resources/"
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Kotoba</string>
  <key>CFBundleDisplayName</key><string>Kotoba</string>
  <key>CFBundleIdentifier</key><string>com.pingfanhu.kotoba</string>
  <key>CFBundleVersion</key><string>${VERSION}</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundleExecutable</key><string>Kotoba</string>
  <key>CFBundleIconFile</key><string>Kotoba</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSMinimumSystemVersion</key><string>14.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSHumanReadableCopyright</key><string>MIT</string>
  <key>LSUIElement</key><true/>
  <key>CFBundleDevelopmentRegion</key><string>en</string>
  <key>CFBundleLocalizations</key><array><string>en</string><string>zh-Hans</string><string>zh-Hant</string><string>ja</string><string>ko</string><string>fr</string><string>de</string><string>es</string></array>
  <key>NSMicrophoneUsageDescription</key><string>Kotoba listens while you dictate. What you say is turned into words on this Mac and sent nowhere.</string>
</dict>
</plist>
PLIST
codesign --force --deep --sign - "$APP"
echo "built $APP ($VERSION)"
if [[ "${1:-}" == "--run" ]]; then
  open "$APP"
fi
