# Probing

How to look at a state of the window from a script.

## Rules

- Probe with a copy that has its own `KOTOBA_HOME` at a scratch folder. Link `models` there to `~/.kotoba/models` to use the models already downloaded. With a home of its own it leaves the person's Kotoba running (`sys::end_other_kotobas`); a Kotoba opened without one ends every other, a probe copy run as the dev bundle too.
- A copy run with `KOTOBA_GO` or `KOTOBA_SHOT` keeps its window over every other app's (`sys::keep_on_top`): a covered window draws nothing and runs no step, and macOS no longer lets a background app step in front.
- A synthetic click does not reach a click handler in a background window. Say what was checked and what was not.
- A click posted with Control in its flags leaves Control on the next click posted from the same event source, which the menu bar item then takes for a right click. Set every posted click's flags.
- A bare `target/debug/Kotoba` run from a terminal has the terminal's permissions, not its own. Look at permissions in the bundle (`scripts/dev-app.sh`, which passes `KOTOBA_*` through).

## Launches

```
KOTOBA_HOME=<scratch> KOTOBA_SETTINGS=general ./target/debug/Kotoba        # open the settings on a section (general, models, customize, about)
KOTOBA_SHOT=<file.png> KOTOBA_SHOT_AFTER=4 ...                           # a picture of the window, then quit
KOTOBA_GO="history;close;section:customize"                                 # steps, the first a second in, then five seconds apart
```

The picture is taken from inside (`sys::shoot_window`): a process may capture its own window without Screen Recording access.

`KOTOBA_GO` steps (`Kotoba::go_step`):

- `history`, `terms`, `avoid`, `close`: the sheets. On a list's sheet, `copyall` is Copy All, `deleteall` Delete All and `deleteyes` its confirmation.
- `keys:<keystroke>`: a key pressed in the window, through its bindings (`keys:cmd-w` closes it).
- `wheel:<x>:<y>:<dy>`: the wheel turned there, in points from the window's top left, through the window as the mouse sends it.
- `words:<terms|avoid>:<words>`: words typed into a list's field and added, as Enter does (a comma for a space). `unword:<terms|avoid>`: the list's first word taken out.
- `pick:dictate-key`, `pick:dictate-keep`, `pick:mic`: the lists that drop down. `group:<one|two|f>`: on More's list, a kind's keys opened beside it.
- `fstate:<0|1>`: F1 to F12 read as doing what is printed on them (0) or as standard function keys (1), in place of the Mac's setting.
- `get`: the lightest speech model not here, downloaded. `clean:<on|off>`: post-processing turned on or off.
- `dark`, `light`: the appearance. `section:<key>`: another section.
- `untrusted`: the permissions drawn as if neither were given.
- `notice:<words_with_underscores>`: a notice.
- `fnuse:<n>`: Fn's use read as `n` (0 is Do Nothing) in place of the Mac's. `dictatekey:<key>`: the key set (`dictatekey:tap:fn`). `reset`.
- `key`: dictation's key pressed. `dictate:pretend`: the pill listening to a made-up voice. `pillshot:<png>`: a picture of the pill's window.
- `dictate:file:<wav>`: a 16-bit WAV written down by the model and delivered as the microphone's recording is (with a field of the window focused, into that field). `say -o x.aiff "…"` and `afconvert -f WAVE -d LEI16@16000 x.aiff x.wav` make one.
- `away`: prints whether Accessibility is given, the tap on, and the microphone's status.
- `KOTOBA_LANG=<code>` (an environment variable, not a step): the window in that language (`fr`, `de`, `zh-Hant`, …) in place of the system's. `language:<system|code>`: the language list's choice, as a press on it (in the bundle it changes the app's own `AppleLanguages`; set it back with `language:system`). `pick:language`: the list open.
- `bar`: prints the menu bar item's frame, from the main screen's top left as a click is posted, and whether it is shown. `barswap:<on|off>` and `baricon:<show|hide>`: Swap clicks and Show in menu bar. `login:<on|off>`: Launch at login (only in the bundle, `scripts/dev-app.sh`; it changes the Mac's Login Items). `menushot:<png>`: the item's menu shown, and a picture of each of the app's windows taken while it is (`<png>` named `-<n>.png`, the menu the `NSPopupMenuWindow`); the capture draws the menu's glass as flat gray.

`KOTOBA_TAP_DEBUG=1` prints every key, click and change of the modifiers the tap watch is handed.

## The core from a terminal

`kotoba-core dictate models` lists both kinds of model and whether each is here; `dictate download [key]` fetches one as the settings do (a post-processing key too); `dictate file <wav> [key]` prints the words; `clean <key> <words | file>` runs post-processing on words, or on a file a dictation a line, and prints what it wrote and whether it was taken. The microphone is the app's alone: a process a terminal started is the terminal's in the system's eyes.
