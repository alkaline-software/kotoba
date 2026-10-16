# Dictation

About `crates/kotoba-core/src/dictate.rs`, `clean.rs`, `native/llm.cpp`,
`build.rs`, and in the app `kotoba.rs` (running it), `pill.rs`, and the
tap, paste and sounds in `sys.rs`. Lifted from Emaki, where
`docs/composer.md` ("Dictation" to "In other apps") has the longer story.

## Rules

- A speech model and a post-processing model are let go before the
  process exits (`dictate::unload`, `clean::unload`, at quit in
  `main.rs` and in the CLI). Metal's device is torn down at exit, and
  ggml aborts the process if a model still holds its memory then.
- No model ships with the app, in the bundle or the repository: each is
  downloaded by the person from Settings, Models, into
  `~/.kotoba/models`. With none there, the key opens that section and
  nothing listens.
- Every bundle carries `NSMicrophoneUsageDescription` (`dev-app.sh`,
  `make-app.sh`, and `crates/kotoba-app/macos/Info.plist` merged by
  cargo-packager), and the notarized one is signed with the audio-input
  entitlement (`macos/Kotoba.entitlements`). Without the line the system
  ends an app that opens the microphone; without the entitlement, under
  the hardened runtime, it hands the app silence.
- libllama runs on transcribe.cpp's ggml (`vendor/llama.cpp/UPSTREAM.md`).
  Two copies of ggml in one program clash.
- Whisper is never handed a recording with no voice (`speech_span`).
- Post-processing is taken only when it keeps to what was said
  (`clean::accept`).
- Nothing here waits on the settings window: the work runs on the app's
  clock, and the window may be closed throughout.

## The models

Four speech models (Qwen3-ASR 1.7B and 0.6B, Whisper Large v3 Turbo and
Small), each hearing many languages and two mixed in one sentence, run
on the GPU by transcribe.cpp. Three post-processing models (Qwen3.5 4B,
2B, 0.8B) run by libllama. A download goes on from where it was cut off,
at a fixed commit, and the file takes its name only once its SHA-256 is
the published one. Any number download at once (`Kotoba::download_model`),
each on its own thread. A model is loaded on first use and let go after
ten minutes unused (`release_idle`, on a one-minute clock in
`Kotoba::new`).

## A dictation

The key (`app.dictate_key`, ⌘⇧D unless the settings say another) or a
tap of modifiers alone starts it (`toggle_dictation`):

1. Two soft notes rising, and the microphone opened 300 ms later so the
   chime is not heard. The pill comes down from the top of the screen.
2. Listening: the pill's bars follow the voice (`tick_dictation`).
3. The key again: two notes falling; the pill's bars run as a wave while
   the model writes (`write_down`). What was heard is spelled with the
   terms, post-processed when that is on, kept in the history, and
   delivered (`deliver_dictated`).

**Where the words go** is decided when they are ready. In another app
they are pasted: the general pasteboard is set (marked
`org.nspasteboard.TransientType`, which clipboard managers skip) and ⌘V
posted, and what it held is put back 600 ms later unless something else
was copied meanwhile. In Kotoba's own window the field with the keyboard
takes them as typed (`dispatch_keystroke`, deferred until the update is
over and sent through the window, never its leased root view). Without
Accessibility, or with no field to take them, they are left on the
clipboard and the settings say so.

**Escape** while the microphone starts or listens cancels with no chime
(`cancel_dictation`): in Kotoba's window by a capture-phase action, in
other apps by the tap, which keeps Escape from that app only then.
Escape while the model writes does nothing: stopping is the choice to
keep the words.

**A recording of exact zeros** is the system refusing the microphone,
which it does with silence and not an error: the pill says "No sound"
and the settings open on General, where the permissions are.

## Keys in other apps

`sys::outside_keys` is a `CGEventTap` at the session level, active, on
the main run loop. It sees modifier changes (for a tap), dictation's key
when it is a key (kept from that app, matched by its key code on the US
layout) and Escape while listening. It re-enables itself when the system
disables it for a timeout, and it ignores everything while Kotoba is in
front, where AppKit's local monitor (`sys::key_events`) and gpui's
binding see the keys. Both need nothing more than Accessibility, and
nothing on a Mac sees another app's keys without it.

**Modifiers alone are chosen, not pressed**, from the list under More
(`dictate::TAPS`): Right ⌘, ⌥, ⇧, ⌃, Fn and the pairs. A tap is the keys
held between nothing held and nothing held, with no key or click among
them (`dictate::TapWatch`).

**Fn has a job of the Mac's own** unless the person gives it up
(Keyboard settings, "Press 🌐 key to"). With Fn chosen and a job set, a
line under the key's row says what Fn would also do, with a button to
Keyboard settings; it is read again whenever the window comes back
(`read_fn_use`).

## Post-processing, preferences and terms

As Emaki has them (`../emaki/docs/composer.md`, Post-processing and
Preferences and terms). Preferences are the person's own words, first
in the system turn. Terms are added several at once, handed to the
speech model as vocabulary and spelled as saved by `dictate::spell`,
before post-processing and again after. The 4B does not follow a
preference the examples show against (no punctuation in Chinese, digits
in prose); that is the model's limit.

**Avoid** (`app.dictate_avoid`) is the person's list of words never to
be written, kept and added as terms are. `dictate::avoid` leaves each
out wherever it stands as a whole word, in any capitals, with the comma
it leaves at a clause's start or end, the extra space, and the capital a
sentence began with. It runs on what was heard, after `spell` and before
post-processing sees it, and again on what the model wrote, so it holds
with post-processing off; the history keeps what was heard whole.

## The history

What the model writes is kept for as long as the settings say
(`app.dictate_keep`): not at all, a day, a week (the default), a month, a
year or always, a line of JSON a dictation in
`~/.kotoba/dictation/history.jsonl`, with what was heard beside it when
post-processing changed it. A shorter time trims the file at once. Show
History, in General, opens it on a sheet over the settings.

## The pill

`pill.rs`, Emaki's whole: a black pill hanging from the top edge of the
screen under the pointer, the wave at its left and the voice at its
right, as wide as the notch and a side each on a screen with one. It is
gpui's PopUp kind made borderless (`sys::float_pill`), so the system lets
it stand over the menu bar, never takes the keyboard or the pointer, and
is not hidden when Kotoba is not in front. The notch layout has not yet
been looked at on a notched screen.
