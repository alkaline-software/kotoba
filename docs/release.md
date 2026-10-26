# Releasing Kotoba

What to run, in order, to cut a release, and what to do when a step fails.

## What a release is

A `v*` tag pushed to `alkaline-software/kotoba` runs `release.yml`. Two package jobs build the app for Apple silicon and Intel on the Apple silicon runner, then one `release` job creates the GitHub Release as a draft, with the tag's section of `CHANGELOG.md` as its notes, and attaches the disk images under stable names:

| asset | built by | signed |
|---|---|---|
| `Kotoba-mac-arm64.dmg`, `Kotoba-mac-x64.dmg` | CI, then replaced from the signer's Mac | Developer ID, notarized, stapled |

The names never change, so `releases/latest/download/<name>` links stay valid. Kotoba is for macOS alone: no release carries anything for Windows or Linux.

## One-time setup on a Mac

- Rust via rustup, plus `rustup target add x86_64-apple-darwin`.
- `cargo install cargo-packager --locked`.
- CMake (`brew install cmake`), for transcribe.cpp.
- Xcode or its command line tools (`codesign`, `hdiutil`, `notarytool`, `stapler`).
- A `Developer ID Application` certificate in the login keychain (`security find-identity -v -p codesigning` lists it); `KOTOBA_SIGN_IDENTITY` names one when there are several.
- The notarytool keychain profile `notarytool-profile` (`xcrun notarytool store-credentials notarytool-profile` once, with the Apple ID, an app-specific password and the team ID); `NOTARY_PROFILE` names another.
- `gh auth login` with write access to the repository.
- For the disk image's background: Python with Pillow (`uv run --with pillow …`) and LXGW WenKai Medium in `~/Library/Fonts`.

## Each release

### 1. Bump the version

`version = "X.Y.Z"` in the root `Cargo.toml`; both crates inherit it. CI builds with `--locked`, so refresh the lock and commit it with the rest:

```
cargo check -p kotoba-app
```

### 2. Write the changelog section

`## vX.Y.Z` at the top of `CHANGELOG.md`, for the person installing the app: a numbered list, a topic in bold and one sentence, small fixes under **Fixes**, ten items at most. `scripts/release-notes.sh X.Y.Z` prints what the release page will show.

### 3. Check it before the tag

Commit the bump and the changelog, then:

```
caffeinate -d -t 14400 &             # the display stays on, so the screen stays unlocked
scripts/release-check.sh X.Y.Z
```

It holds the commit to everything the tag will be held to: the version, the changelog section, the notary credential, a lock that builds `--locked`, a clean tree, no tag of that name yet, the tests, and the app build. The notary credential is out of reach while the screen is locked (When something fails); an agent starts `caffeinate` as a background command of its own. Do not tag until the check says "ready to tag".

### 4. Push, wait for the tests, then tag

Work happens on a branch named `vX.Y.Z-<what it is>`, never the tag's own name; `main` is fast-forwarded to it at release. `rust.yml` runs on a push to `main`, and it is the only place the tests run on a clean Mac:

```
git push origin HEAD                 # the branch
git push origin HEAD:main            # fast-forward main; starts rust.yml
gh run list --repo alkaline-software/kotoba --workflow rust --limit 1
gh run watch <run-id> --repo alkaline-software/kotoba --exit-status
git tag -a vX.Y.Z -m "Kotoba vX.Y.Z"
git push origin refs/tags/vX.Y.Z
```

The tag starts `release.yml`, about twenty minutes, ending in a draft: nothing is public, and `releases/latest`, which the app's update check follows, still names the previous version. Watch it with `gh run list --repo alkaline-software/kotoba --workflow release --limit 3` and `gh run watch`.

### 5. Build, sign and notarize the Mac images here

From the tagged commit, while CI runs: `release-publish.sh` refuses an image older than the tag's commit.

```
cargo build --release --locked -p kotoba-app
cargo build --release --locked -p kotoba-app --target x86_64-apple-darwin
scripts/release-mac.sh --no-build --notarize
scripts/release-mac.sh --no-build --notarize --target x86_64-apple-darwin
```

Each run ends with `built dist/Kotoba-mac-<arch>.dmg (X.Y.Z, <arch>, signed, notarized)`. Check each run's exit code and last line, not output piped through `tail`.

### 6. Put them on the release and publish it

```
scripts/release-publish.sh X.Y.Z
```

It refuses until the release workflow for the tag has succeeded (`action-gh-release` replaces same-named assets, so an upload made while the job runs is overwritten) and both images are notarized, stapled and newer than the tag's commit; then uploads them over CI's, downloads them back and compares checksums, checks them as Gatekeeper will, and publishes the draft as the latest release. The release is not done until it prints "published with notarized Mac images".

### 7. Install the release build here

```
hdiutil attach -nobrowse dist/Kotoba-mac-arm64.dmg
ditto "/Volumes/Kotoba vX.Y.Z/Kotoba.app" /Applications/Kotoba.app.new
mv /Applications/Kotoba.app "$TMPDIR/Kotoba.app.old" && mv /Applications/Kotoba.app.new /Applications/Kotoba.app
rm -rf "$TMPDIR/Kotoba.app.old"
hdiutil detach "/Volumes/Kotoba vX.Y.Z"
```

Beside the old bundle and renamed into place, never copied over it: the installed app may be running, and a copy over a running app's executable can end it. The release (`com.pingfanhu.kotoba`) has its own entries in Privacy & Security, apart from the dev build's (`com.pingfanhu.kotoba.dev`); a later release signed with the same Developer ID keeps them.

### 8. Delete the compiled files

```
scripts/release-clean.sh
```

Every time, without being asked, once step 6 printed its line and the installed copy opens. It deletes `target/` (every build, and with it the dev build's bundle) and `dist/`, fails if Spotlight finds a second `Kotoba.app`, and refuses while the installed copy is not a notarized release or `dist/` holds images of a version not yet published. When the dev build is running, it hands over to the installed copy once the turn is over (`scripts/relaunch.sh --installed`): say so in the reply. The next build starts from nothing and takes some minutes.

## Redoing a release under the same tag

Delete the release and the tag on both sides, cancel a run still going (`gh run cancel <run-id>`), then repeat from step 3 with the new commit; the Mac images are rebuilt and published again:

```
gh release delete vX.Y.Z --yes --repo alkaline-software/kotoba
git tag -d vX.Y.Z
git push origin :refs/tags/vX.Y.Z
```

An agent's auto mode refuses these as destructive unless the person allowed them; otherwise the person runs them.

## Signing in CI

With five repository secrets, `release.yml` signs and notarizes the Mac images itself; step 5 goes, and step 6 is `gh release edit vX.Y.Z --draft=false --latest`:

```
security export -k login.keychain-db -t identities -f pkcs12 -P "<passphrase>" -o /tmp/cert.p12
gh secret set MACOS_CERTIFICATE_P12 --repo alkaline-software/kotoba < <(base64 -i /tmp/cert.p12)
gh secret set MACOS_CERTIFICATE_PASSWORD --repo alkaline-software/kotoba --body "<passphrase>"
gh secret set APPLE_ID --repo alkaline-software/kotoba --body "<Apple ID email>"
gh secret set APPLE_APP_PASSWORD --repo alkaline-software/kotoba --body "<app-specific password>"
gh secret set APPLE_TEAM_ID --repo alkaline-software/kotoba --body "<team id>"
rm /tmp/cert.p12
```

## The disk image

`release-mac.sh` makes a read-write image, mounts it, and has Finder set the 660 by 420 window, the 128 pt icons at 165 and 495, and the background, then converts it to a compressed read-only image. Only Finder writes a `.DS_Store` Finder honors, so the layout is not generated offline; this works on the GitHub macOS runners too. The background is `scripts/dmg/background.png`, 1320 by 840 at 2x, drawn by `scripts/dmg/background.py` in the app's cream and green ("Drag Kotoba to Applications" and an arrow). The PNG is committed; run the script only when the picture changes. The app icon and the menu bar image are in `docs/look.md`.

## When something fails

Every failure met while cutting a release gets a line here once it is solved, with what would have caught it, and the steps are changed so it is caught before the tag next time.

- **A test passes here and fails in CI**: something on this Mac (a model downloaded, a setting) is not on a clean runner. Wait for `rust.yml` before tagging (step 4).
- **One Mac package job fails right after "satisfies its Designated Requirement", with no message**: `hdiutil create` on the runner ("Resource busy"). `release-mac.sh` tries five times and says why; if it still fails, re-run only that job, which keeps the tag and the images built here: `gh run rerun <run-id> --failed --repo alkaline-software/kotoba`.
- **Finder "AppleEvent timed out (-1712)"**: macOS asked whether the terminal may control Finder and nobody answered. Allow it (Privacy & Security, Automation) and run again, then check that both images are in `dist/`.
- **"No Keychain password item found for profile: notarytool-profile"** while the profile is in place: the screen is locked, and the profile's keychain opens only while it is unlocked. Keep the display on (step 3) and notarize while the person is at the Mac.
- **"src refspec vX.Y.Z matches more than one"**: a branch has the tag's name. Push the tag by its full name, `refs/tags/vX.Y.Z`, as step 4 does. Once the tag exists, Git calls such a branch `heads/vX.Y.Z`, and a Git app lists it as a second, unpublished branch. Name the branch `vX.Y.Z-<what it is>`.
- **"Cargo.toml says X but the tag is vY"**: fix `Cargo.toml`, commit, redo the tag.
- **"CHANGELOG.md has no section '## vX.Y.Z'"**: add it, commit, redo the tag.
- **`--locked` fails in CI after a bump**: `Cargo.lock` was not refreshed. `cargo check -p kotoba-app`, commit the lock.
- **Notarization rejected**: `xcrun notarytool log <submission-id> --keychain-profile notarytool-profile`; look for a binary without the hardened runtime or an unsigned helper the packager added.
- **The Finder layout step fails**: the volume `Kotoba vX.Y.Z` was already mounted, or Finder had no session. The script detaches a stale volume first; `hdiutil info` shows what is mounted.
- **The Mac images on the release are ad-hoc signed**, or macOS says the app "cannot be opened" after an update: step 6 was skipped. Run `scripts/release-publish.sh X.Y.Z`. `spctl -a -vv /Applications/Kotoba.app` must say "Notarized Developer ID"; `Signature=adhoc` from `codesign -dv` is CI's image.
- **A release left as a draft**: the app's update check finds nothing, since `releases/latest` names only published releases. Step 6 is the release.
- **Two Kotobas in Launchpad or Spotlight**: `mdfind "kMDItemFSName == 'Kotoba.app'"`; keep only `/Applications/Kotoba.app`. The dev bundle lives in `target/debug/dev.noindex/`, which Spotlight passes by, and step 8 removes the rest.
- **"Node.js NN is deprecated" on a run**: an action is a major version behind. `gh api repos/<owner>/<action>/releases/latest -q .tag_name` gives the current one; move `rust.yml` and `release.yml` together.
