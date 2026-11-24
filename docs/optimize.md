# Optimization

About keeping Kotoba fast and light: the three aims of a pass, how one is done, the rules passes have left, and the log that makes each pass start where the last one stopped. The studio's method is the standards' `docs/optimize.md`; this page is Kotoba's own part of it.

## Rules

- **A pass has three aims:** the best speed; the least use of the computer (CPU, memory, power, wakeups, disk, network, processes); and nothing Kotoba does lost. The third is never traded for the other two: a change that saves time and changes what a person sees, hears or can do, or a word a model writes, is not made.
- **Measured before and after,** the same way both times, with a stopwatch in a copy or a throwaway test, taken out when done. The numbers go in the log. The first guess is often wrong: a change that looked right saved nothing until it was timed.
- **Incremental.** A pass reads the log's last entry and looks only at what changed since the commit it names (`git diff <commit>..`), plus anything it left for later. With no entry, the pass covers everything. A pass that is not logged is lost to the next one.
- **Kotoba mostly waits.** It sits in the menu bar all day with its window closed, so what it costs at rest counts more than what it costs at work: no timer turns while nothing moves, and what turns anyway (the minute's `release_idle` and the daily look for an update) does little at each turn. A clock that draws (the pill's wave at `WAVE_TICK`) stops when what it draws is gone. The toolkit's clocks count as ours: gpui-component's notification list ticked every 50 ms for as long as the window was open, with no notification ever shown (`vendor/gpui-component/UPSTREAM.md`).
- **The main thread never waits.** The event tap is on it, so a wait there holds every key and click on the Mac, in every app. A model's lock, a sound, an Accessibility call and a file go to a thread of their own (`docs/dictation.md`, `docs/control.md`).
- **Nothing slow is asked while a frame is built.** A hover draws the window whole, so what `render` and each row call runs at every hover: what the system has is found once, at launch (`fonts::install`, `docs/look.md`), and a list that grows with use draws what is near its box (the history's `said_in_view`, `docs/window.md`). A row's call runs once a row a frame; count what a row asks, not only what the view asks once.
- **A model is loaded while the person speaks, and held only while it is used.** The key's first press starts the loading (`prepare_models`), so the time it takes is spent while the person talks and not after the second press. The two models together hold 2.8 GB; each is let go after ten minutes unused (`IDLE_MODEL`), off the main thread. Nothing loads after the quit's `unload`, so a load begun ahead never outlasts it (`docs/dictation.md`).
- **What is the same each time is worked out once.** The start of post-processing's prompt is read once and kept while it is unchanged (`kotoba_llm_complete`, `docs/dictation.md`): it was three quarters of a short dictation's time. Time a model with `llama_synchronize`, since `llama_decode` returns before the GPU is done.
- **What moves on its own is the system's to draw, at the rate it needs.** Control's frame is Core Animation's (`glow.rs`), drawn with no frame of ours, which the model would wait behind. Whatever moves in a window has WindowServer draw at the screen's rate, about a quarter of a core on a 5K screen at 60 Hz, however small the window and whatever moves; a motion too slow for a frame more to be seen asks for fewer (`Pace::Drift`, `preferredFrameRateRange`). Time a change on screen in WindowServer's CPU (`ps -o time= -p <WindowServer>`), not only in Kotoba's.

## A pass

1. Read the log's last entry: the commit it covers to, and what it left for later. With no entry, take everything.
2. List what is new since its commit (`git log`, `git diff`): the files, and the parts of the window, the pill and the frame they draw.
3. Find the hot paths in it: what `render` and each row call (a disk read, a system call, a lock, a copy of a list, a parse); clocks and timers (does anything wake while nothing moves, or while the window is closed); the main thread and the tap (does either wait); loops off it, and what each turn costs; what grows with use (memory, a model kept, the history drawn whole, files kept under `~/.kotoba`).
4. Time each suspect as it is now. At rest: `top -pid <pid> -stats cpu,idlew,mem` with the window closed and open, CPU over 20 seconds from `ps -o time=`, memory from `footprint`. A dictation's parts: `kotoba-core dictate file <wav>` and `clean <key> <file>` from a terminal (`docs/probing.md`, The core from a terminal). A frame: the time from the top of `render` to a `canvas` painted last. A wait on the main thread: a 5 ms timer there that prints any gap over 30 ms. Where the time goes: `sample` on the dev build, which keeps its symbols. Each thing timed twice running, and again after a pause of several seconds, since a quiet Mac takes a moment to wake its sound and GPU.
5. Fix the cause, in the place every caller goes through.
6. Time it again, the same way.
7. Check that nothing is lost: `cargo test -p kotoba-core -p kotoba-app`, the same recordings through `kotoba-core` giving the same words, and a probe copy of each part changed, in each state it has (`docs/probing.md`).
8. Write the log entry.

## Log

Newest first. Each entry: the version and date, the commits covered (`<last entry's commit or tag>..` the commit that adds the entry), what was looked at, what was found and done with the numbers before and after in a table, what was looked at and left as it is and why, and what was left for later.

### v0.1.6, 2026-11-24, second pass

`8354b00..` the commit that adds this entry: control's frame and sound redesigned, and the settings' presses that move nothing (rows of one height, `same_room`, notices in the top bar). A release build of `b561c35` against a release build of this pass, each run as a probe copy with its own home, the settings window open. The frame timed over 20 s spans in WindowServer's CPU and Kotoba's, with the frame off, at rest (listening, no voice) and working, on a 5K Studio Display at 60 Hz; each span's cost is what it adds over the frame off. The other apps of the Mac were running, so WindowServer's baseline moved between 2.4 and 3.8 s.

| | Before | This pass |
|---|---|---|
| Control's frame at rest, WindowServer's CPU over 20 s above the frame off | 5.5 to 6.1 s (27 to 30% of a core) | 2.5 to 2.6 s (12 to 13%) |
| The frame working, the same (unchanged by this pass; the spread is the other apps') | 5.1 to 5.9 s | 6.1 to 6.3 s |
| Kotoba's own CPU with the frame shown, over 20 s | 0.1 to 0.2 s | 0.1 to 0.2 s |

Found and fixed:
- The colors' slow turn while control listens (12 degrees a second) was drawn at the screen's rate, and with it WindowServer at 60 frames a second the whole time the frame was up. It now asks for 15 (`Pace::Drift`). Two pictures 5 s apart show the same turn. Asking for up to 20 saved less (2.2 to 3.0 s), and the cost does not fall in step with the rate, so fewer than 15 would save little more.

Looked at and left as they are:
- The frame working costs WindowServer about 30% of a core. The marks slide 260 points a second and need every frame: at 30 a mark jumps 9 points a frame. The cost is WindowServer drawing at the screen's rate whatever moves: a window 40 points tall cost the same as one the size of the screen, the marks alone without the band the same (7.8 s against 8.1), and the wheel of colors drawn at an eighth of its resolution the same.
- `glow::level`, each 80 ms while control listens, on the main thread: 1.4 ms (2.2 at the 90th percentile) for the 36 layers' stroke ends, 1.7% of the thread. The time is Core Animation's commit of 72 implicit animations; setting them without key-value coding was no faster. A key pressed meanwhile waits at most 2.5 ms.
- The settings at rest, after the changes to presses: 0.17 to 0.20 s of CPU over 45 s with the window open and nothing moving (0.4% of a core), and with it closed none, no idle wakeups and 18 MB, as at the last pass. `presence` sets one timer as something closes and none while it is open; `same_room` lays out both words and animates nothing at rest.
- Control's two sounds are 70 KB each, carried in the program and played as dictation's are.

Left for later: nothing.

### v0.1.6, 2026-11-24, first pass

The first pass, over everything: from the repository's start to the commit that adds this entry. A release build of `c8c40ed` against a release build of this pass, each run as a probe copy with its own home and the models of this Mac linked in (Qwen3-ASR 1.7B, Qwen3.5 4B with post-processing on, the person's own instructions and 12 terms). Dictation's stages timed from a throwaway example over the core on a WAV of 9.8 s, one sentence.

| | Before | This pass |
|---|---|---|
| The first dictation after launch or ten idle minutes, from the second press to the words, after 3 s of speaking | 2.79 to 2.92 s | 1.00 to 1.02 s |
| The same after 1 s of speaking | 2.9 s | 1.9 s |
| A later dictation | 0.97 to 1.00 s | 0.95 to 1.00 s |
| The settings window open, nothing moving, CPU over 45 s | 0.20 to 0.34 s (0.7% of a core) | 0.07 to 0.20 s (0.4%) |
| The installer left in `~/.kotoba/cache` after an update | 11 MB | none |

Found and fixed:
- The models were loaded at the key's second press, so the first dictation after launch or a while unused waited for them: 0.5 s for the speech model and 1.3 s for the 4B and the fixed start of its prompt, after the person had stopped speaking. They are now loaded at the first press, off the main thread (`prepare_models`, `dictate::prepare`, `clean::prepare`, `control::prepare`, `kotoba_llm_prepare`). The words written are the same, word for word. With speech shorter than the loading, the dictation waits for what is left; the speech model running beside the 4B's loading is slower then (1.3 s against 0.44), and the whole still shorter. A dictation stopped by Escape leaves the models loaded until the ten minutes are up.
- A load begun ahead could start after the quit had let the models go, which ends in ggml's abort. `unload` now closes each engine: nothing loads after it.
- gpui-component's notification list ran its 50 ms clock for the settings window's life; it now runs while there is a notification, and Kotoba shows none. What the window costs at rest now is gpui's display link stepping it, which draws nothing while nothing changed.
- The installer's disk image stayed in the cache after an update; it is deleted once the install is over.

Looked at and left as they are:
- At rest with the window closed: 0% CPU, no idle wakeups, 19 MB. The one clock is the minute's (`release_idle`, the daily update check).
- The event tap, called for every key and click in every app: two atomics, one `isActive`, a send on a channel; on the main thread a match against the two tap keys. No disk, no lock.
- The Models page asks the disk whether each model is here (`installed`, `chosen`) at every frame: 11 µs a frame for its seven rows.
- The history, at a year of this Mac's rate (8,000 dictations, 5.9 MB) kept forever: read in 4 to 8 ms as its sheet opens, once, and read again by `remember` to prune, 4 to 10 ms off the main thread a dictation. Pruning by the first line alone would keep an older line after the clock was set back.
- The microphone takes about 515 ms to start, 450 of it the device waking in `play()` (the Studio Display's microphone). Opened under the chime, the wait would be hidden, but the chime would be in what it hears and a Bluetooth headset would change its mode while the chime plays. The models loading beside it do not slow it: 513 to 515 ms with them, 516 to 536 without.
- Qwen3-ASR 1.7B holds 2.3 GB while loaded, for a file of 1.5 GB: transcribe.cpp reads the weights into buffers of its own and has no option to map the file. The 4B maps its weights from the file and holds 650 MB of its own. Let go, the process goes back from 2.8 GB to 116 MB.
- The pill while it listens: 2.6% of a core, a frame every 80 ms for its bars, most of it in Metal's and Core Animation's threads. `to_whisper`: 0.3 ms for 10 s of sound.

Left for later: nothing.
