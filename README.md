# Kotoba

言葉, "words". Dictation anywhere on your Mac: press a shortcut in any
app, speak, and the words are written where the cursor is. A speech
model on your Mac writes down what you said, and, when you turn it on, a
small language model tidies it up while keeping to what you said.
Nothing you say leaves the machine.

Kotoba is a menu bar app by [Alkaline Software](https://github.com/alkaline-software),
the dictation of [Emaki](https://github.com/alkaline-software/emaki) in an
app of its own.

## Install

Download `Kotoba-mac-arm64.dmg` (Apple silicon) or `Kotoba-mac-x64.dmg`
(Intel) from the [latest release](https://github.com/alkaline-software/kotoba/releases/latest),
open it, and drag Kotoba to Applications. It updates itself from
Settings, Updates.

## Use

1. Open Kotoba. Its leaf is in the menu bar: a click opens the
   settings, a right click shows its menu. The first time, it opens
   the settings on **General**: allow the microphone (to hear you)
   and Accessibility (for the shortcut in any app, and to paste there).
2. In **Models**, download a speech model. Qwen3-ASR is best with
   speech that mixes languages; Whisper hears the most languages.
3. Press the shortcut (⌘⇧D, or another key or a modifier tapped alone,
   such as Right ⌘, under More), speak, and press it again. Esc cancels
   while it listens.

Post-processing is in **Models**. **Customize** holds your Preferences
for it, the Terms written exactly as you save them, and the words to
Avoid. The history of what you said is in **General**.

## How it is built

Rust and [GPUI](https://www.gpui.rs), Zed's toolkit, with
[gpui-component](https://github.com/longbridge/gpui-component).
Speech runs on [transcribe.cpp](https://crates.io/crates/transcribe-cpp),
post-processing on llama.cpp's library built against the same ggml.
Dictation runs on a Mac only for now.

## Development

```
cargo test -p kotoba-core
cargo build -p kotoba-app && scripts/dev-app.sh
```

Building needs CMake (`brew install cmake`). `AGENTS.md` says how the
pieces fit and where each part's notes are; `WORKFLOW.md` is the release.

## License

MIT
