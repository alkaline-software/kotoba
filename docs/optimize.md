# Optimization

About keeping Kotoba fast and light: the three aims of a pass, how one is done, the rules passes have left, and the log that makes each pass start where the last one stopped. The studio's method is the constitution's `docs/optimize.md`; this page is Kotoba's own part of it.

## Rules

- **A pass has three aims:** the best speed; the least use of the computer (CPU, memory, power, wakeups, disk, network, processes); and nothing Kotoba does lost. The third is never traded for the other two: a change that saves time and changes what a person sees, hears or can do, or a word a model writes, is not made.
- **Measured before and after,** the same way both times, with a stopwatch in a copy or a throwaway test, taken out when done. The numbers go in the log. The first guess is often wrong: a change that looked right saved nothing until it was timed.
- **Incremental.** A pass reads the log's last entry and looks only at what changed since the commit it names (`git diff <commit>..`), plus anything it left for later. With no entry, the pass covers everything. A pass that is not logged is lost to the next one.
- **Kotoba mostly waits.** It sits in the menu bar all day with its window closed, so what it costs at rest counts more than what it costs at work: no timer turns while nothing moves, and what turns anyway (the minute's `release_idle` and the daily look for an update) does little at each turn. A clock that draws (the pill's wave at `WAVE_TICK`) stops when what it draws is gone. The toolkit's clocks count as ours: gpui-component's notification list ticked every 50 ms for as long as the window was open, with no notification ever shown (`vendor/gpui-component/UPSTREAM.md`).
- **The main thread never waits.** The event tap is on it, so a wait there holds every key and click on the Mac, in every app. A model's lock, a sound, an Accessibility call and a file go to a thread of their own (`docs/dictation.md`, `docs/control.md`).
- **Nothing slow is asked while a frame is built.** A hover draws the window whole, so what `render` and each row call runs at every hover: what the system has is found once, at launch (`fonts::install`, `docs/look.md`), and a list that grows with use draws what is near its box (the history's `said_in_view`, `docs/window.md`). A row's call runs once a row a frame; count what a row asks, not only what the view asks once.
- **A model is loaded while the person speaks, and held only while it is used.** The key's first press starts the loading (`prepare_models`), so the time it takes is spent while the person talks and not after the second press. The two models together hold 2.8 GB; each is let go after ten minutes unused (`IDLE_MODEL`), off the main thread. Nothing loads after the quit's `unload`, so a load begun ahead never outlasts it (`docs/dictation.md`).
- **What is the same each time is worked out once.** The start of post-processing's prompt is read once and kept while it is unchanged (`kotoba_llm_complete`, `docs/dictation.md`): it was three quarters of a short dictation's time. Time a model with `llama_synchronize`, since `llama_decode` returns before the GPU is done.
- **What moves on its own is the system's to draw.** Control's frame is Core Animation's (`glow.rs`), drawn at the screen's rate with no frame of ours, which the model would wait behind.

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

### v0.1.6, 2026-11-24

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
