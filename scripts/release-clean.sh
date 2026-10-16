#!/bin/zsh
# The end of a release: delete everything compiled. target/ (every build
# made while developing and for the release) and dist/ (the two disk
# images, which are on the release by now), then look for a Kotoba.app
# other than the installed one. All of it is rebuilt from the source.
#
# The dev build may be running. Its files go with target/ and it keeps
# running, since a running program holds what it has open; a few seconds
# later it is quit and /Applications/Kotoba.app is opened in its place
# (scripts/relaunch.sh --installed). So the installed copy must be a
# notarized release, or this refuses.
#
#   scripts/release-clean.sh
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"
root="$PWD"
fail() { print -u2 "release-clean: $1"; exit 1; }

# ps, not pgrep: pgrep leaves out its own ancestors. In Emaki, whose
# agents run inside the dev build, it reported nothing with the build up.
running=$(ps -axo pid=,command= | awk -v t="$root/target/" 'index($0, t) && $2 != "awk"' || true)
spctl -a -vv /Applications/Kotoba.app 2>&1 | grep -q "Notarized Developer ID" \
  || fail "/Applications/Kotoba.app is not a notarized release (WORKFLOW.md step 7)"

# Images still in dist/ are a release's. A release not yet published may
# be redone, and a redo wants them and the release builds.
if [[ -n "$(print -l dist/*.dmg(N))" ]]; then
  v=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)
  draft=$(gh release view "v$v" --repo alkaline-software/kotoba --json isDraft -q .isDraft 2>/dev/null || true)
  [[ "$draft" == false ]] || fail "dist/ holds disk images and v$v is not published (WORKFLOW.md step 6)"
fi

before=$(du -sh . | cut -f1)
cargo clean
rm -rf dist
print "release-clean: $before before, $(du -sh . | cut -f1) now"

others=$(mdfind "kMDItemFSName == 'Kotoba.app'" | grep -vx "/Applications/Kotoba.app" || true)
[[ -z "$others" ]] || fail "another Kotoba.app is on the disk; Launchpad and Spotlight list each one:
$others"
print "release-clean: only /Applications/Kotoba.app is left"
if [[ -n "$running" ]]; then
  print "release-clean: the build that is running has no files left; the installed one takes over"
  scripts/relaunch.sh --installed
fi
