# Platform

About `vendor/`, `Cargo.toml`, `Cargo.lock`, `.github/workflows/`, `scripts/`, `crates/kotoba-core/src/update.rs`, `config.rs`, `paths.rs`, and `crates/kotoba-app/src/sys.rs`.

## Rules

- **Kotoba is for macOS alone**, on Apple silicon and Intel. Nothing is gated for another system and nothing is built for one: the code calls AppKit, Core Audio and Metal directly. Windows and Linux builds were dropped in v0.1.4, and their installers taken off every earlier release.
- `vendor/llama.cpp` is built against transcribe-cpp's ggml and must come from the llama.cpp commit that ggml was synced from (`vendor/llama.cpp/UPSTREAM.md`). Update the two together.
- Build and test from the repository's root, never inside `vendor/gpui-component`: it is a workspace of its own, and cargo there resolves its own lock against another gpui.
- Never put a `rev` on our own gpui dependency. gpui-component depends on the unpinned zed git source, and a `rev` makes a second copy of gpui. The pin is in `Cargo.lock` (zed `ce48461e…`): `cargo update -p gpui --precise <rev>` moves it.
- Every change under `vendor/gpui-component/` is marked in the source and listed in `vendor/gpui-component/UPSTREAM.md`. The copy is Emaki's, with Emaki's changes in it, marked `(Emaki addition.)`.
- Nothing personal is in the repository. `release-mac.sh` and `dev-app.sh` take the first Developer ID identity in the keychain, or `KOTOBA_SIGN_IDENTITY`; the notary credential is a keychain profile.
- The `.icns` is the full square, opaque to its corners: macOS 26 masks every app icon itself, and anything transparent shows as a gray border.
- Leave no second `Kotoba.app` where Spotlight indexes. The dev build's bundle is in a folder named `.noindex` for this.

## Data

`~/.kotoba` (`KOTOBA_HOME` overrides, as tests and probes do): `config.json` with every setting under `app`, `models/`, `dictation/history.jsonl`, `state/update.json`. Kotoba never reads `~/.emaki`.

## Building

Building needs CMake (`brew install cmake`) for transcribe.cpp, and the Intel target (`rustup target add x86_64-apple-darwin`) for the Intel image.

## CI and releases

`docs/release.md` is the procedure. `rust.yml` runs the tests, an app build and the CLI on a macOS runner at every push to `main` and every pull request; `release.yml` builds both disk images on a `v*` tag into a draft release. The notarized images are built on this machine (`scripts/release-mac.sh`) and uploaded over CI's (`scripts/release-publish.sh`).

## Self-update

`update.rs`: the newest version is the tag `releases/latest` redirects to, the disk image for the Mac's chip is fetched under its stable asset name, mounted, `Kotoba.app` copied beside the running bundle and swapped in, and the new app opened once this one has gone. `Kotoba::run_check` and `install_update_now` run it on threads; the daily check runs from the app's one-minute clock when `app.check_updates` is on, and a newer version found puts a dot on the rail's About, where the updates are.

## The bundles

`scripts/dev-app.sh` starts the build as `target/debug/dev.noindex/Kotoba.app` (`com.pingfanhu.kotoba.dev`), opened through the system with a Dock launch's environment and signed with the developer's identity, so a permission given to it outlives a rebuild. The release is `com.pingfanhu.kotoba`: each is its own entry in Privacy & Security.
