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
KOTOBA_HOME=<scratch> KOTOBA_SETTINGS=general ./target/debug/Kotoba        # open the settings on a section (general, dictation, control, models, about)
KOTOBA_SHOT=<file.png> KOTOBA_SHOT_AFTER=4 ...                           # a picture of the window, then quit
KOTOBA_GO="history;close;section:control"                                   # steps, the first a second in, then five seconds apart
```

The picture is taken from inside (`sys::shoot_window`): a process may capture its own window without Screen Recording access.

`KOTOBA_GO` steps (`Kotoba::go_step`):

- `history`, `terms`, `avoid`, `close`: the sheets. On a list's sheet, `copyall` is Copy All, `deleteall` Delete All and `deleteyes` its confirmation.
- `keys:<keystroke>`: a key pressed in the window, through its bindings (`keys:cmd-w` closes it).
- `click:<x>:<y>`: the pointer moved there and pressed, in points from the window's top left, through the window, so the click's handler runs. Opening a list shows nothing of what a choice on it does: press the choice, and read `config.json`.
- `wheel:<x>:<y>:<dy>`: the wheel turned there, in points from the window's top left, through the window as the mouse sends it.
- `words:<terms|avoid>:<words>`: words typed into a list's field and added, as Enter does (a comma for a space). `unword:<terms|avoid>`: the list's first word taken out.
- `pick:dictate-key`, `pick:dictate-keep`, `pick:mic`: the lists that drop down. `group:<one|two|f>`: on More's list, a kind's keys opened beside it.
- `fstate:<0|1>`: F1 to F12 read as doing what is printed on them (0) or as standard function keys (1), in place of the Mac's setting.
- `hide`: Kotoba stepped back from the front, as it is when the person works elsewhere. Do this first when control acts on another app: a probe's window floats over it, and takes what is read and scrolled. `say:<words_with_underscores>`: words followed by control, as if heard; the steps are real. `control:file:<wav>`: a recording followed, through the speech model. `look:<file>`: what the app in front shows, as an agent reads it. `control`: control's key pressed. `agent:<claude|codex|none>`: the Agent row. `pick:control-key`: its More. `glow:on`, `glow:off`, `glow:think`, `glow:level:<0 to 1>`: the light; `glowshot:<png>`: a picture of its window, with its alpha, to lay over a ground.
- `get`: the lightest speech model not here, downloaded. `clean:<on|off>`: post-processing turned on or off.
- `dark`, `light`: the appearance. `section:<key>`: another section.
- `untrusted`: the permissions drawn as if neither were given.
- `notice:<words_with_underscores>`: a notice.
- `fnuse:<n>`: Fn's use read as `n` (0 is Do Nothing) in place of the Mac's. `dictatekey:<key>`: the key set (`dictatekey:tap:fn`). `reset`.
- `escape`: Escape, as a press of it stops a dictation. `key`: dictation's key pressed. `dictate:pretend`: the pill listening to a made-up voice. `pillshot:<png>`: a picture of the pill's window.
- `dictate:file:<wav>`: a 16-bit WAV written down by the model and delivered as the microphone's recording is (with a field of the window focused, into that field). `say -o x.aiff "…"` and `afconvert -f WAVE -d LEI16@16000 x.aiff x.wav` make one.
- `away`: prints whether Accessibility is given, the tap on, and the microphone's status.
- `checking`, `uptodate`, `checkfailed`, `found`, `downloading`, `installing`, `installfailed`: each state the Updates button comes to, drawn; nothing is fetched. `check` is the button's own press, a real check.
- `loginwait` (Launch at login waiting on the person in Login Items), `micrefused` (the microphone refused), `keytaken` (dictation's key refused as taken), `dlerror` (a model's download stopped short): the states a press leads to that a probe cannot press its way to.
- `KOTOBA_LANG=<code>` (an environment variable, not a step): the window in that language (`fr`, `de`, `zh-Hant`, …) in place of the system's. `language:<system|code>`: the language list's choice, as a press on it (in the bundle it changes the app's own `AppleLanguages`; set it back with `language:system`). `pick:language`: the list open.
- `bar`: prints the menu bar item's frame, from the main screen's top left as a click is posted, and whether it is shown. `barswap:<on|off>` and `baricon:<show|hide>`: Swap clicks and Show in menu bar. `login:<on|off>`: Launch at login (only in the bundle, `scripts/dev-app.sh`; it changes the Mac's Login Items). `menushot:<png>`: the item's menu shown, and a picture of each of the app's windows taken while it is (`<png>` named `-<n>.png`, the menu the `NSPopupMenuWindow`); the capture draws the menu's glass as flat gray.

`KOTOBA_TAP_DEBUG=1` prints every key, click and change of the modifiers the tap watch is handed. `KOTOBA_AGENT_LOG=<file>` keeps all an agent said and did in a run.

Steps run only with a window: give `KOTOBA_SETTINGS`, or `KOTOBA_SHOT` never fires and the copy never quits. Step back with `hide`, and act in the bundle (`scripts/dev-app.sh`), which holds Accessibility.

## A press, checked

A press moves nothing but what it changes (`docs/design.md`, A press moves nothing), and a picture of each state alone does not show that: a row that grew by one line looks finished on its own. Each state a press leads to is shot beside the state before it, at the same section, language and appearance, from a home whose `config.json` is put back before each shot (a step such as `agent:claude` writes it), and compared:

```
KOTOBA_HOME=<scratch> KOTOBA_SETTINGS=about KOTOBA_SHOT=<dir>/rest.png KOTOBA_SHOT_AFTER=3 ./target/debug/Kotoba
KOTOBA_HOME=<scratch> KOTOBA_SETTINGS=about KOTOBA_GO=uptodate KOTOBA_SHOT=<dir>/uptodate.png KOTOBA_SHOT_AFTER=3 ./target/debug/Kotoba
Rscript scripts/press-diff.R <dir>/rest.png <dir>/uptodate.png
```

It prints, for the rail and the page apart, the box in points that holds every pixel that differs, leaving out the traffic lights, which differ with whether the window has the keys. The box must sit inside the control pressed (the Updates button, and Release notes before it once a newer Kotoba is found), in the line under a row's name for what it says, in the window's top for a notice, or in the rail for a dot. A box that reaches a row's or a card's edge, or anything under the control, is a press that moved the layout. A probe whose window was covered at the step runs no step and shows no difference: shoot it again. Done in English and in German and Japanese, whose words run longest; the widths that hold them were measured with the text system (`window.text_system().shape_line`) over every word each control can show, in every language.

## A corner, checked

A sharp corner is a few pixels at a box's corner and does not show in a picture of the whole window. Each rounded box's corners are cut from the probe's picture and enlarged eight times with no smoothing, in both appearances:

```
Rscript scripts/corners.R <dir>/models.png <dir>/corners.png speech-tl=226,80 speech-tr=804,80
```

Each `name=x,y` is a 20-point square from its top left, in points. A corner is round in the strip: nothing of what is inside the box (a washed row, a hover, a row cut by a scroll, a scroll bar) shows square past the curve. A list's corners are shot with a row cut at its edge and the pointer on it (`history;wheel:430:548:-230` cuts one at the history sheet's foot).

## The core from a terminal

`kotoba-core dictate models` lists both kinds of model and whether each is here; `dictate download [key]` fetches one as the settings do (a post-processing key too); `dictate file <wav> [key]` prints the words; `clean <key> <words | file>` runs post-processing on words, or on a file a dictation a line, and prints what it wrote and whether it was taken; `control <key> <words | file> [app]` prints the steps for what was said, a line each, and carries out none. The microphone is the app's alone: a process a terminal started is the terminal's in the system's eyes.
