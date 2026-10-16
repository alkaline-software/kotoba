#!/bin/sh
# Start the build in target/debug as the system starts an app, not as a
# shell starts a program.
#
#   scripts/dev-app.sh [arguments for `open`]
#
# On a Mac a bare executable is nobody's app. Run from a shell it is the
# terminal's child: the system asks for the microphone and Accessibility
# on the terminal's behalf, and keeps the answer under the terminal's
# name. It has no Info.plist either, so it is not a menu bar app
# (LSUIElement) and has no line to show when it asks for the microphone.
# So the build goes in a bundle of its own,
# target/debug/dev.noindex/Kotoba.app, with the identifier
# com.pingfanhu.kotoba.dev (its own entry in Privacy & Security, apart
# from the installed Kotoba's), and is opened through the system with an
# environment that holds only what a Dock launch has.
#
# The folder's name ends in ".noindex", which Spotlight passes by: in
# target/debug itself the bundle was indexed, and Launchpad and Spotlight
# listed a second Kotoba beside the installed one.
#
# The bundle is signed with a signing identity of the developer's when the
# keychain has one (KOTOBA_SIGN_IDENTITY, else the first "Developer ID
# Application"). macOS keeps what an app was allowed (the microphone,
# Accessibility) by who signed it; a build signed by nobody is a new app
# each time it is built, and the system asks again at every launch after
# a build. With no identity it is signed by nobody and does ask.
# The executable is a copy, since signing writes into the file.
#
# Elsewhere the build is simply run.

cd "$(dirname "$0")/.." || exit 1
bin=target/debug/Kotoba
[ -x "$bin" ] || { echo "no $bin: cargo build -p kotoba-app" >&2; exit 1; }
[ "$(uname)" = Darwin ] || exec "./$bin"

app=target/debug/dev.noindex/Kotoba.app
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
rm -f "$app/Contents/MacOS/Kotoba"
cp -c "$bin" "$app/Contents/MacOS/Kotoba" 2>/dev/null || cp "$bin" "$app/Contents/MacOS/Kotoba"
cp -f crates/kotoba-app/assets/icon/icon.icns "$app/Contents/Resources/Kotoba.icns"
# The microphone's question in each language the window speaks.
cp -R crates/kotoba-app/macos/locales/. "$app/Contents/Resources/"
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Kotoba</string>
  <key>CFBundleDisplayName</key><string>Kotoba</string>
  <key>CFBundleIdentifier</key><string>com.pingfanhu.kotoba.dev</string>
  <key>CFBundleVersion</key><string>0</string>
  <key>CFBundleShortVersionString</key><string>dev</string>
  <key>CFBundleExecutable</key><string>Kotoba</string>
  <key>CFBundleIconFile</key><string>Kotoba</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSMinimumSystemVersion</key><string>14.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>LSUIElement</key><true/>
  <key>CFBundleDevelopmentRegion</key><string>en</string>
  <key>CFBundleLocalizations</key><array><string>en</string><string>zh-Hans</string><string>zh-Hant</string><string>ja</string><string>ko</string><string>fr</string><string>de</string><string>es</string></array>
  <key>NSMicrophoneUsageDescription</key><string>Kotoba listens while you dictate. What you say is turned into words on this Mac and sent nowhere.</string>
</dict>
</plist>
PLIST

identity="${KOTOBA_SIGN_IDENTITY:-$(security find-identity -v -p codesigning 2>/dev/null | sed -n 's/.*"\(Developer ID Application: [^"]*\)".*/\1/p' | head -1)}"
codesign --force --sign "${identity:--}" "$app" >/dev/null 2>&1 || codesign --force --sign - "$app" >/dev/null 2>&1

# `open` hands the app the opener's environment, so the opener's is made
# the one the Dock would give: no shell's PATH, no terminal's marks.
# KOTOBA_* go through, for a probe copy.
set -- "$@" "$app"
exec env -i \
    HOME="$HOME" USER="$USER" LOGNAME="${LOGNAME:-$USER}" SHELL="${SHELL:-/bin/zsh}" \
    PATH=/usr/bin:/bin:/usr/sbin:/sbin \
    TMPDIR="$(getconf DARWIN_USER_TEMP_DIR)" \
    $(env | grep -E '^KOTOBA_[A-Z_]+=' | grep -v ' ' ) \
    /usr/bin/open -n "$@"
