#!/bin/zsh
# Everything a tag will be held to, checked here before it is pushed: the
# version in Cargo.toml and Cargo.lock, the changelog section, the notary
# credential, a clean tree, the tests and the build. Each of these is
# something that has failed a release after its tag was out (a lock not
# refreshed), and a tag that fails is a tag redone, with the Mac images
# built and notarized again.
#
#   scripts/release-check.sh 0.1.6
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"
v="${1:?version, e.g. 0.1.6}"
fail() { print -u2 "release-check: $1"; exit 1; }

grep -q "^version = \"$v\"$" Cargo.toml || fail "Cargo.toml does not say version = \"$v\""
scripts/release-notes.sh "$v" > /dev/null || fail "CHANGELOG.md has no section for v$v"
# The release page is read by people updating: a short numbered list.
lines=$(scripts/release-notes.sh "$v" | grep -c '^[0-9][0-9]*\. ' || true)
(( lines >= 1 )) || fail "the v$v changelog section has no numbered item"
(( lines <= 10 )) || fail "the v$v changelog section has $lines items; ten at most"

# The notary service is asked now, with what release-mac.sh will use: a
# credential it cannot reach is found here, not after the tag is out. The
# keychain profile is out of reach while the screen is locked, so keep the
# display on for a release (caffeinate -d).
if [[ "$(uname)" == Darwin ]]; then
  if [[ -n "${APPLE_ID:-}" ]]; then
    notary=(--apple-id "$APPLE_ID" --password "${APPLE_APP_PASSWORD:-}" --team-id "${APPLE_TEAM_ID:-}")
  else
    notary=(--keychain-profile "${NOTARY_PROFILE:-notarytool-profile}")
  fi
  said=$(xcrun notarytool history "${notary[@]}" 2>&1) \
    || fail "the notary service cannot be reached with the saved credential (docs/release.md, When something fails):
${said##*Error: }"
fi

# --locked is how CI builds: a lock that was not refreshed fails here.
cargo check --locked -p kotoba-app -p kotoba-core 2>&1 | tail -1
[[ -z "$(git status --porcelain)" ]] || fail "uncommitted changes; the tag would not hold them"
git rev-parse -q --verify "refs/tags/v$v" > /dev/null && fail "the tag v$v exists already (docs/release.md, redoing a release)"

cargo test --locked -p kotoba-core 2>&1 | grep -E "^test result|FAILED|panicked" || true
cargo test --locked -p kotoba-core > /dev/null 2>&1 || fail "the core tests fail"
cargo test --locked -p kotoba-app > /dev/null 2>&1 || fail "the app's tests fail"
cargo build --locked -p kotoba-app 2>&1 | tail -1
print "release-check: v$v is ready to tag at $(git rev-parse --short HEAD)"
