# Platform

About `vendor/`, `Cargo.toml`, `Cargo.lock`, `.github/workflows/`,
`scripts/`, `crates/kotoba-core/src/update.rs`, `config.rs`, `paths.rs`,
and the `cfg` gates in `crates/kotoba-app/src/sys.rs`.

## Rules

- `vendor/llama.cpp` is built against transcribe-cpp's ggml and must
  come from the llama.cpp commit that ggml was synced from
  (`vendor/llama.cpp/UPSTREAM.md`). Update the two together.
- Build and test from the repository's root, never inside
  `vendor/gpui-component`: it is a workspace of its own, and cargo there
  resolves its own lock against another gpui.
- Never put a `rev` on our own gpui dependency. gpui-component depends on
  the unpinned zed git source, and a `rev` makes a second copy of gpui.
  The pin is in `Cargo.lock` (zed `ce48461e…`, the revision Emaki
  builds against): `cargo update -p gpui --precise <rev>` moves it.
- Every change under `vendor/gpui-component/` is marked in the source and
  listed in `vendor/gpui-component/UPSTREAM.md`. The copy is Emaki's,
  with Emaki's changes in it, marked `(Emaki addition.)`.
- Everything Mac-only is gated with a stated fallback. Dictation is a
  Mac's alone for now (`dictate::AVAILABLE`): elsewhere the settings say
  so, and `main.rs` allows the dead code that leaves.
- The `.icns` is the full square, opaque to its corners: macOS 26 masks
  every app icon itself, and anything transparent shows as a grey border.
- Leave no second `Kotoba.app` where Spotlight indexes. The dev build's
  bundle is in a folder named `.noindex` for this.

## Data

`~/.kotoba` (`KOTOBA_HOME` overrides, as tests and probes do):
`config.json` with every setting under `app`, `models/`,
`dictation/history.jsonl`, `state/update.json`. Kotoba never reads
`~/.emaki`.

## Windows and Linux

They build; dictation does not run there. transcribe.cpp is C++ built by
CMake, which the MinGW check from a Mac cannot build, so its crates are
macOS dependencies. The closest check from a Mac:

```
CARGO_TARGET_DIR=target/xwingnu CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc \
  cargo check -p kotoba-app -p kotoba-core --target x86_64-pc-windows-gnu
```

Building on a Mac needs CMake (`brew install cmake`) for transcribe.cpp.

## CI and releases

WORKFLOW.md is the procedure. `rust.yml` runs the core tests, an app
build and the CLI on all three OSes at every push; `release.yml` builds
installers with cargo-packager on a `v*` tag into a draft release. The
notarized Mac images are built on this machine (`scripts/release-mac.sh`)
and uploaded over CI's (`scripts/release-publish.sh`).

## Self-update

`update.rs`, Emaki's: the newest version is the tag `releases/latest`
redirects to, the installer is fetched under the stable asset names, and
on a Mac the disk image is mounted, `Kotoba.app` copied beside the
running bundle and swapped in, and the new app opened once this one has
gone. `Kotoba::run_check` and `install_update_now` run it on threads; the
daily check runs from the app's one-minute clock when
`app.check_updates` is on, and a newer version found puts a dot on the
rail's Updates.

## The bundles

`scripts/dev-app.sh` starts the build as
`target/debug/dev.noindex/Kotoba.app` (`com.pingfanhu.kotoba.dev`),
opened through the system with a Dock launch's environment and signed
with the developer's identity, so a permission given to it outlives a
rebuild. The release is `com.pingfanhu.kotoba`: each is its own entry in
Privacy & Security.
