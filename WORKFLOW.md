# Releasing Kotoba

Everything about cutting a release: bumping the version, the changelog,
the tag, what CI builds, signing and notarizing the Mac images on this
machine, uploading them, cleaning up, redoing a release, and the pieces
that feed it (the icon, the menu bar image, the disk-image background).
AGENTS.md says how the pieces fit; this file says what to run, in order.
It is Emaki's release with the names changed. The versions named below
as lessons are Emaki's: each step that guards against something is there
because it went wrong in an Emaki release.

## What a release is

A `v*` tag pushed to `alkaline-software/kotoba` runs `release.yml`. Four
package jobs build the app on macOS (both architectures, from the Apple
Silicon runner), Windows and Linux, then one `release` job creates the
GitHub Release, as a draft, with the tag's section of `CHANGELOG.md` as
its notes and attaches the installers under stable names:

| asset | built by | signed |
|---|---|---|
| `Kotoba-mac-arm64.dmg` | CI, then replaced from this machine | Developer ID, notarized, stapled |
| `Kotoba-mac-x64.dmg` | CI, then replaced from this machine | Developer ID, notarized, stapled |
| `Kotoba-windows-x64-setup.exe` | CI | unsigned (SmartScreen asks once) |
| `Kotoba-linux-x64.AppImage` | CI | n/a |
| `Kotoba-linux-x64.deb` | CI | n/a |

The names never change, so `releases/latest/download/<name>` links stay
valid. CI's own Mac images are ad-hoc signed because the signing secrets
are not in the repository (see "Signing in CI" below); the notarized ones
are built here by `scripts/release-mac.sh` and uploaded over them.
Dictation runs on a Mac alone for now; the Windows and Linux installers
are built all the same, so those builds keep working for when it follows.

## One-time setup on a Mac

- Rust via rustup, plus the Intel target: `rustup target add x86_64-apple-darwin`.
- `cargo install cargo-packager --locked`.
- Xcode or its command line tools (`codesign`, `hdiutil`, `notarytool`, `stapler`).
- The certificate `Developer ID Application: Pingfan Hu (XC2WL5WN7J)` in the
  login keychain (`security find-identity -v -p codesigning` lists it).
- The notarytool keychain profile `notarytool-profile`
  (`xcrun notarytool store-credentials notarytool-profile` once, with the
  Apple ID, an app-specific password and team `XC2WL5WN7J`).
- `gh auth login` with an account that has write access to the repository.
- For the Windows type check in `scripts/release-check.sh`:
  `brew install mingw-w64` and `rustup target add x86_64-pc-windows-gnu`.
- For the disk-image background only: Python with Pillow and the LXGW
  WenKai Medium font in `~/Library/Fonts`. The icon needs nothing beyond
  a Mac (`python3` and `swift` ship with it).

## Each release

### 1. Bump the version

The version lives in one file:

```
Cargo.toml            version = "X.Y.Z"      (the workspace; both crates inherit it)
```

`Cargo.lock` records the crates' versions too, and CI builds with
`--locked`, so refresh it after the bump and commit it with the rest:

```
cargo check -p kotoba-app
```

The release workflow refuses a tag whose number disagrees with `Cargo.toml`.

### 2. Write the changelog section

Add `## vX.Y.Z` to `CHANGELOG.md`, newest first, written for the person
installing the app: what changed for them, not which files moved. Keep it
short: one line per change, a dozen lines at most, small fixes grouped
into one. The
workflow refuses a tag with no section, and `scripts/release-notes.sh X.Y.Z`
prints exactly what the release page will show.

### 3. Check it before the tag

Commit the bump and the changelog, then:

```
scripts/release-check.sh X.Y.Z
```

It holds the commit to everything the tag will be held to: the version in
`Cargo.toml`, a changelog section of a dozen lines at most, a notary
service that answers to the saved credential (step 5 needs it, an hour
later), a lock that builds with `--locked`, a clean tree, the core tests,
the app build, and a Windows type check (MinGW; `brew install mingw-w64 && rustup target add
x86_64-pc-windows-gnu` once). Do not tag until it says "ready to tag". A
tag that fails in CI has to be redone, and every redo means building and
notarizing the Mac images again; Emaki's v0.1.5 was tagged three times,
the first for a line that did not compile on Windows, which this check
catches in a minute.

### 4. Push, wait for the tests, then tag

Work happens on a branch; `main` is fast-forwarded to it at release.
`rust.yml` runs on a push to `main`, and it is the only place the tests
run on Windows and Linux (the check in step 3 type-checks Windows and
runs nothing there). So push, let that run finish, and tag only once it
is green on all three:

```
git push origin HEAD                 # the branch
git push origin HEAD:main            # fast-forward main; starts rust.yml
gh run list --repo alkaline-software/kotoba --workflow rust --limit 1
gh run watch <run-id> --repo alkaline-software/kotoba --exit-status
git tag -a vX.Y.Z -m "Kotoba vX.Y.Z"
git push origin vX.Y.Z
```

The Mac images (step 5) can be built while it runs: the commit is the
one that will be tagged, unless the tests send you back.

The tag push starts `release.yml`. It takes about twenty minutes (the
cold GPUI build), and ends with a **draft** release holding CI's
installers: nothing is public, and `releases/latest` (what an updater
follows) still names the previous version. Watch it:

```
gh run list --repo alkaline-software/kotoba --workflow release --limit 3
gh run watch <run-id> --repo alkaline-software/kotoba --exit-status
```

### 5. Build, sign and notarize the Mac images here

While CI runs, and from the tagged commit: an image built before the last
fix is an image of other code, and `release-publish.sh` refuses one older
than the tag's commit. Both architectures build on an Apple Silicon Mac.
The script packages the bundle with cargo-packager, signs it with the
Developer ID and the hardened runtime, lays the disk image out through
Finder over `scripts/dmg/background.png`, compresses it, signs it, submits
it to the notary service, waits, staples and validates:

```
cargo build --release --locked -p kotoba-app
cargo build --release --locked -p kotoba-app --target x86_64-apple-darwin
scripts/release-mac.sh --no-build --notarize
scripts/release-mac.sh --no-build --notarize --target x86_64-apple-darwin
```

Each run ends with `built dist/Kotoba-mac-<arch>.dmg (X.Y.Z, <arch>, signed,
notarized)` and leaves only the two images in `dist/`. Notarization takes a
few minutes per image; the four commands together about ten.

Check each run's own exit code and last line. Step 3 asked the notary
service already, so a credential that cannot be reached here is new since
then: see "When something fails".

### 6. Put them on the release and publish it

Once the workflow is green and both images are built:

```
scripts/release-publish.sh X.Y.Z
```

It refuses unless the release workflow for the tag has finished and
succeeded (`action-gh-release` replaces same-named assets, so an upload
made while the job runs is overwritten by it) and both images are
notarized, stapled and newer than the tag's commit. Then it uploads them
over CI's ad-hoc ones, downloads them back and compares checksums, checks
the downloaded copies the way Gatekeeper will, and publishes the draft as
the latest release. The release is not done until this prints "published
with notarized Mac images". Emaki's v0.1.5 was published by CI with the
ad-hoc images and the upload came half an hour later; the app updated
itself from them in between and macOS refused it.

### 7. Install the release build here

Drag from the disk image, or:

```
hdiutil attach dist/Kotoba-mac-arm64.dmg
ditto "/Volumes/Kotoba vX.Y.Z/Kotoba.app" /Applications/Kotoba.app
hdiutil detach "/Volumes/Kotoba vX.Y.Z"
```

Kotoba is a menu bar app: it opens with no window and no Dock icon,
only its icon in the menu bar. The release build has its own entries in
Privacy & Security (Microphone, Accessibility), apart from the dev
build's (`com.pingfanhu.kotoba.dev`), so its first launch asks for both.
A later release, signed with the same Developer ID, keeps them.

Keep no other `Kotoba.app` on the disk (`dist/`, a scratch folder,
`scripts/make-app.sh`'s output): Launchpad and Spotlight list each one.
Step 8 removes the ones a release leaves.

### 8. Delete the compiled files

Every release ends with this step, and a release is not finished until it
has run. Nothing compiled is kept between releases: disk space is worth
more here than the minutes the next build takes. Do it without being
asked, once the release is confirmed done: step 6 printed "published with
notarized Mac images" and the installed copy opens. Then:

```
scripts/release-clean.sh
```

- It runs `cargo clean`, which deletes `target/`: every build made while
  developing and for the release (debug, release, the Intel and Windows
  targets), and with it the dev build's bundle. Months of incremental
  builds had grown Emaki's past 80 GB by its v0.1.9.
- It deletes `dist/`, the two disk images, which are on the release by
  now.
- It asks Spotlight for every `Kotoba.app` and fails if there is one other
  than `/Applications/Kotoba.app`.

It refuses in two cases: the installed copy is not a notarized release,
or `dist/` holds images of a version that is not published (a redo of the
release wants them and the release builds).

Nothing here can be lost: all of it is rebuilt from the source. The cost
is time. The next `cargo build -p kotoba-app` starts from nothing and takes
some minutes, and so does the next release build.

The dev build (`scripts/dev-app.sh`) may be running, with its bundle in
`target/`. The script cleans under it all the same: a running program
keeps the files it has open, so it goes on. It has no bundle on disk from
then on, so the script ends by handing over: `scripts/relaunch.sh
--installed` quits it a few seconds later and opens
`/Applications/Kotoba.app` in its place.

Afterwards `du -sh .` on the repository should be well under a gigabyte;
the script prints it.

## Redoing a release under the same tag

Delete the release and the tag on both sides, cancel a run still going
(`gh run cancel <run-id>`), then repeat from step 3 with the new commit:

```
gh release delete vX.Y.Z --yes --repo alkaline-software/kotoba
git tag -d vX.Y.Z
git push origin :refs/tags/vX.Y.Z
git tag -a vX.Y.Z -m "Kotoba vX.Y.Z"
git push origin vX.Y.Z
```

The Mac images must be rebuilt from the new commit (step 5) and published
again (step 6).

## Signing in CI

`release.yml` signs and notarizes on the runner when five repository
secrets exist; step 5 then disappears and step 6 is only
`gh release edit vX.Y.Z --draft=false --latest`. Write access is enough to
set them:

```
security export -k login.keychain-db -t identities -f pkcs12 -P "<passphrase>" -o /tmp/cert.p12
gh secret set MACOS_CERTIFICATE_P12 --repo alkaline-software/kotoba < <(base64 -i /tmp/cert.p12)
gh secret set MACOS_CERTIFICATE_PASSWORD --repo alkaline-software/kotoba --body "<passphrase>"
gh secret set APPLE_ID --repo alkaline-software/kotoba --body "<Apple ID email>"
gh secret set APPLE_APP_PASSWORD --repo alkaline-software/kotoba --body "<app-specific password>"
gh secret set APPLE_TEAM_ID --repo alkaline-software/kotoba --body "XC2WL5WN7J"
rm /tmp/cert.p12
```

Windows stays unsigned until SignPath (free for open source) is set up.

## The pieces a release depends on

**The icon.** `scripts/icon/icon.html` is the source: it draws the picture
on a canvas, at any size. `scripts/make-icon.sh` runs it (WebKit, through
`scripts/icon/render.swift`, so on a Mac) into `crates/kotoba-app/assets/icon`:
the PNGs as the rounded icon with its margin and shadow (the Dock icon the
app sets at start, Windows, Linux), `icon.ico`, and `icon.icns` as the full
square, opaque to the corners, because macOS 26 masks every app icon to its
own rounded square over a grey backing and shows anything transparent as a
grey border. The same page draws the menu bar's template image: the leaf
in black on transparent with four bars cut out, which the system
tints. The script writes it to `crates/kotoba-app/assets/icons`,
`menubar.png` at 36 px (18 pt at 2x) and `menubar-1x.png` at 18. Open the
page in a browser to see every size on one sheet. Change the drawing, run
the script, rebuild, commit the assets.

**The disk-image background.** `scripts/dmg/background.png`, drawn by
`scripts/dmg/background.py` (Pillow, LXGW WenKai Medium) in the app's cream
and green: "Drag Kotoba to Applications" and an arrow, 1320x840 at 2x for
the 660x420 Finder window. Run the script only when the picture changes
(`uv run --with pillow python3 scripts/dmg/background.py` if the Python at
hand has no Pillow); the PNG is committed.

**The layout.** `release-mac.sh` mounts a read-write image and has Finder
set the window, the 128pt icons at 165 and 495, and the background, then
converts to a compressed read-only image. Only Finder writes a `.DS_Store`
Finder honours, so the layout is not generated offline. This works on the
GitHub macOS runners too.

## When something fails

Every failure met while cutting a release gets a line here once it is
solved, with what would have caught it, and the steps above are changed
so it is caught before the tag next time.

- **A test passes here and fails on Windows or Linux** (the `rust`
  workflow goes red after the push): the tests only run on the other
  systems in CI. Step 4 now waits for that run before tagging. Emaki's
  v0.1.6 second cut was tagged with it still running, and a new test
  failed on Windows, where a file name's trailing dot is ignored, so `main.rs.`
  answered as `main.rs`; the tag and both notarized images were redone.

- **One Mac package job in `release.yml` fails right after "satisfies
  its Designated Requirement", with no message, while the other
  passes**: `hdiutil create` on the runner ("Resource busy").
  `release-mac.sh` now tries it five times and says why. If it still
  fails, re-run only that job, which keeps the tag and the images built
  here: `gh run rerun <run-id> --failed --repo alkaline-software/kotoba`.
  The `release` job waits for all four. Emaki's v0.1.9 Intel job did this.
- **`release-mac.sh` stops with "Finder ... AppleEvent timed out
  (-1712)"**: macOS asked whether the terminal may control Finder and
  nobody answered. Allow it (System Settings, Privacy & Security,
  Automation) and run the script again. Check the script's exit code and
  that both images are in `dist/`: piped into `tail`, a failed first
  image went unnoticed for Emaki's v0.1.9 until the second was done.

- **`release-mac.sh --notarize` stops with "No Keychain password item
  found for profile: notarytool-profile"**, run by an agent, while the
  same two commands work in the person's own terminal: the credential is
  in place and could not be reached from the agent's session at that
  moment. Why is not known; an hour later the same session reached it.
  Run the two commands of step 5 in a terminal of your own, then go on
  from step 6. `release-check.sh` now asks the notary service before
  anything is built, so step 3 says so. In Emaki's v0.2.3 it was found
  after the tag was out.

- **"Cargo.toml says X but the tag is vY"**: the version and the tag
  disagree. Fix `Cargo.toml`, commit, redo the tag.
- **"CHANGELOG.md has no section '## vX.Y.Z'"**: add it, commit, redo the tag.
- **`--locked` fails in CI after a bump**: `Cargo.lock` was not refreshed.
  `cargo check -p kotoba-app`, commit the lock.
- **Notarization rejected**: `xcrun notarytool log <submission-id>
  --keychain-profile notarytool-profile`. The usual causes are a binary
  signed without `--options runtime` or an unsigned helper inside the bundle;
  the script signs with `--deep` and the hardened runtime, so look for a new
  binary the packager added.
- **The Finder layout step fails**: the volume `Kotoba vX.Y.Z` was already
  mounted, or Finder had no session. The script detaches a stale volume
  first; `hdiutil info` shows what is mounted.
- **Two Kotobas in Launchpad or Spotlight**: a second bundle somewhere on
  disk. `mdfind "kMDItemFSName == 'Kotoba.app'"` lists them; keep only
  `/Applications/Kotoba.app`. The dev build's bundle is made in
  `target/debug/dev.noindex/`, which Spotlight passes by (in Emaki, made
  in `target/debug`, it was listed), and step 8 removes it with the rest
  of `target/`.
- **The Mac images on the release are ad-hoc signed**, or macOS says the
  app "cannot be opened" after an update: step 6 was skipped. Run
  `scripts/release-publish.sh X.Y.Z`. To check an installed copy:
  `spctl -a -vv /Applications/Kotoba.app` must say "Notarized Developer
  ID"; "rejected" with `Signature=adhoc` from `codesign -dv` is CI's
  image. Reinstall from `dist/` (step 7).
- **The Windows job fails with "cannot find type" or an unresolved
  import**: a line used on every platform leans on something gated for
  Unix or macOS. `scripts/release-check.sh` runs the type check that
  finds it; the macOS and Linux jobs pass without it.
- **"Node.js NN is deprecated" on a run**: an action is a major version
  behind. `gh api repos/<owner>/<action>/releases/latest -q .tag_name`
  gives the current one; move `rust.yml` and `release.yml` together.
