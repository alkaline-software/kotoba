# AGENTS.md

## What this is

**Kotoba** (言葉, "words") is a Mac menu bar app for dictation anywhere:
press a shortcut in any app, speak, and the words are pasted where the
cursor is. A speech model on this Mac writes down what was said, and a
small language model, when post-processing is on, tidies it up while
keeping to what was said. Nothing said leaves the machine.

It is Emaki's dictation moved into an app of its own (Emaki is at
`../emaki`, released as v0.2.7). Kotoba keeps Emaki's stack and look:
Rust, GPUI (Zed's toolkit) with the vendored gpui-component, a core
crate with no window and an app crate with the window. It stands on its
own: it never reads `~/.emaki`, and the person downloads its models
again.

## The standard

Kotoba is Emaki's sibling, held to the same standard, and its design is
held to the level of Awwwards, the Webbys and the FWA: minimal, calm,
intuitive, finished in every state (`docs/design.md`). Every change to
the window meets it unasked.

- **Nothing changes from one frame to the next.** What appears, goes,
  opens, closes, grows or moves does so over a moment. A jump is a bug.
  Use the pattern its neighbours use (`docs/window.md`, Motion).
- **Words are few.** Say what to do once, to someone who can use a
  computer. No note that repeats what the control above it shows.
- **Look before saying it is done:** at all the change shows, hides or
  moves, in both appearances (`docs/probing.md`).

## Rules that hold everywhere

Each was learned the hard way in Emaki. The doc named after a rule has
the full story.

- **Models are let go before the process exits** (`dictate::unload`,
  `clean::unload`, at quit in `main.rs` and in the CLI), or every quit
  ends in a crash. (`docs/dictation.md`)
- **One ggml in the program.** libllama is compiled from
  `vendor/llama.cpp` against transcribe-cpp-sys's ggml headers. Never add
  a second copy; update the two together. (`docs/platform.md`)
- **No model ships with the app.** The person downloads each, checked by
  SHA-256.
- **Whisper is never handed a recording with no voice** (`speech_span`),
  and a recording of exact zeros is the system refusing the microphone:
  the settings open where it is allowed. (`docs/dictation.md`)
- **Post-processing is taken only when it keeps to what was said**
  (`clean::accept`). The person's preferences come first in the system
  turn; the examples stay. (`docs/dictation.md`)
- **Terms are spelled as saved** by `spell`, before post-processing and
  again after, and handed to the speech model as vocabulary. **Words to
  avoid are left out** by `avoid`, at the same two places.
- **The event tap is active, at the session level,** and keeps from the
  app in front only dictation's key and Escape while listening. It skips
  events while Kotoba is in front, where the local monitor sees them.
- **The paste marks the pasteboard as transient** and puts the old
  contents back after 600 ms, unless something else was copied meanwhile.
- **The pill is a borderless non-activating panel** that ignores the
  pointer. The notch layout has not been seen on a notched screen yet.
- **Escape while writing does nothing.** Escape while listening cancels
  with no chime.
- **Dictation never waits on a window.** The settings window comes and
  goes; `Kotoba` outlives it, and its work runs on the app's clock
  (`cx.spawn`, never `spawn_in` a window). (`docs/window.md`)
- **Permissions are asked by the person's press, never on a timer.**
  Accessibility is watched (`trust_changes`) and the tap starts the
  moment it is given; whether it is given is whether the tap can be
  made, never `AXIsProcessTrusted`, which a running process reads once.
  A new tap is tried only after Allow or while `AXIsProcessTrusted`
  says yes: a tap made unasked has the system ask.
  (`docs/window.md`, Permissions)
- **A click the window swallows goes through `swallow_click`,** or the
  toolkit's text drag never ends.
- **Every word a person sees goes through `i18n::t`** (or `tf`, `count`)
  and is in `locales/words.json` in every language; `cargo test -p
  kotoba-app` fails otherwise. (`docs/window.md`, Languages)
- **Every button is a pill with an icon** (`ui::choice_pill`,
  `ui::small_pill`), and every key shown goes through `ui::key_text`.
- **One copy of gpui.** The pin is in `Cargo.lock`, not a `rev` of our
  own. Changes to the toolkit go in `vendor/gpui-component/`, marked and
  listed in its `UPSTREAM.md`. (`docs/platform.md`)
- **Everything Mac-only is gated with a stated fallback.** Windows and
  Linux are build targets.

## Before you edit, read

| You are about to touch | Read first |
|---|---|
| `kotoba-core/src/dictate.rs`, `clean.rs`, `native/llm.cpp`, `build.rs`; dictation in `kotoba-app/src/kotoba.rs`; `pill.rs`; the tap, paste and sounds in `sys.rs` | `docs/dictation.md` |
| Anything a person sees: a page, a row, a control, a sheet, the menu | `docs/design.md` |
| `settings.rs`, `controls.rs`, `main.rs`, the window and permissions in `kotoba.rs`, the menu bar in `sys.rs` | `docs/window.md` |
| `look.rs`, `fonts.rs`, `assets.rs`, `ui.rs`, `assets/`, `themes/` | `docs/look.md` |
| `vendor/`, `Cargo.toml`, `.github/workflows/`, scripts, `update.rs`, `config.rs`, `paths.rs`, any `cfg` for an OS | `docs/platform.md` |
| Checking a change in the window from a script | `docs/probing.md` |
| Cutting a release | `WORKFLOW.md` |

Emaki's own docs (`../emaki/docs/composer.md`, `window.md`, `look.md`)
hold the longer history of the parts that came from there.

## Layout

```
Cargo.toml                 workspace; the zed revision is pinned in Cargo.lock
crates/kotoba-core/        everything without a window
  src/dictate.rs             the speech models, downloads, the microphone, transcription, terms, taps, history
  src/clean.rs               post-processing: the small models, the prompt, `accept`
  src/config.rs paths.rs     ~/.kotoba/config.json and the folders under ~/.kotoba
  src/update.rs              the newest release, its installer, and putting it in place
  src/bin/kotoba-core.rs     dictate models | download [key] | file <wav>; clean <key> <words|file>; update
  native/llm.cpp build.rs    libllama on transcribe.cpp's ggml
  tests/core.rs
crates/kotoba-app/         the window
  src/main.rs                the app, its key bindings, the menu bar item's choices
  src/kotoba.rs              the app's one `Kotoba`: dictation, downloads, the key, permissions, updates
  src/settings.rs            the settings window: General, Models, Customize, Updates, About; the history's sheet
  src/controls.rs            switches, lists that drop down, folds, scroll routing
  src/ui.rs                  buttons, keys on caps, hover labels
  src/pill.rs                the pill at the top of the screen
  src/sys.rs                 the Mac's side: menu bar item, event tap, paste, sounds, permissions, screens
  src/i18n.rs                the window's words in the person's language
  locales/words.json         every phrase, by its English, in each language
  src/look.rs fonts.rs assets.rs
  assets/                    icons, sounds, the app icon, the menu bar image
vendor/gpui-component/      Emaki's vendored toolkit, with its UPSTREAM.md
vendor/llama.cpp/           libllama's sources, with its UPSTREAM.md
scripts/                    dev-app.sh, relaunch.sh, make-icon.sh, release-*.sh, dictate-sounds.py
docs/                       how each part works and its traps
WORKFLOW.md                 how to cut a release
CHANGELOG.md                one section per release; the release job reads it
```

## Running it

```
cargo build -p kotoba-app && scripts/relaunch.sh   # quit the running Kotoba, start the new build as an app
scripts/dev-app.sh                                 # start the build as target/debug/dev.noindex/Kotoba.app
```

On a Mac the build runs as a bundle (`com.pingfanhu.kotoba.dev`), signed
with the developer's identity, so the microphone and Accessibility given
to it outlive a rebuild. The release (`com.pingfanhu.kotoba`) is a
separate entry in Privacy & Security, given on its own.

**One Kotoba at a time**, except a probe copy with its own `KOTOBA_HOME`
(`docs/probing.md`). After a change to the app, build it and run
`scripts/relaunch.sh` as the last command.

## Writing things down

- How a part works, a trap, or a fact learned about gpui or macOS goes
  in that part's file under `docs/`, in its `## Rules` when it must not
  be broken.
- This file gets a line only for a rule every session must follow, or a
  new row in the table above.
- Write what is so now and why. What was checked and how belongs in the
  reply to the person, not in a doc.

## Testing

```
cargo test -p kotoba-core
cargo test -p kotoba-app       # the languages' table
cargo build -p kotoba-app
```

`tests/core.rs` points `KOTOBA_HOME` at a temp tree (`isolated()`);
inherit that for anything touching disk. The suite must pass before
committing, and `rust.yml` runs it on all three OSes.
