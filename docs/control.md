# Control

The Mac operated by voice. About `crates/kotoba-core/src/control.rs`, and in the app `act.rs` (the hands), `agent.rs` (the person's agent), `glow.rs` (the frame round the screen), and control's part of `kotoba.rs`.

## Rules

- **The language model is given what was said and the name of the app in front, never what is on the screen.** A page cannot speak to it. A name to click is matched against the screen by `act`, after the model is done.
- **What can be done is the list in `control::Step`.** A line the model writes that is no step refuses the whole plan (`control::plan`). There is no shell, no script, and no address but http and https. An agent's tool calls pass through the same gate (`control::step`), with the same hands.
- **Typing keeps to what was said** (`was_said`): every number spoken, most letters there. The model's plan alone; an agent writes messages of its own.
- **What cannot be taken back is not done:** the keys that empty the Trash and log out are refused (`REFUSED`), and the Apple menu, with Shut Down on it, is left out of what is read.
- **Enter stays the person's to press in a terminal** (`TERMINALS`), once something was typed: a typed line and Enter there is a command run. For an agent, always.
- **Anything sent to another person goes by the agent, which looks first.** The model writes `ask` for what cannot be done blind. With no agent chosen it is not attempted: a plan with no eyes could send to the wrong person.
- **The agent is off unless the person chooses one.** It is the one place what was said leaves the Mac, and what it reads of the screen with it; the row says so.
- **Kotoba never touches an agent's login.** It runs the program the person installed, as published. It reads no token and asks for none.
- **Kotoba's own events carry a mark** (`sys::POSTED`, in `kCGEventSourceUserData`) and the tap lets them through: while control works Escape is taken, and an Esc the plan presses would stop the plan.
- **All of `act` runs off the main thread.** An Accessibility call waits on the app it asks, for as long as that app is busy.
- **`glow` is on the main thread alone,** as AppKit asks.

## One thing said

The key (`app.control_key`, ⌥⌘K unless the settings say another) starts it, as dictation's does (`toggle_control`): the same microphone, but a sound of its own and no pill. Its sound (`control-start.wav`, `control-stop.wav`, from `scripts/control-sounds.R`) is a low F sharp in octaves and then the C sharp above it, the other way round to stop, as loud as dictation's chime and as soft: firmer, brighter or many-noted candidates sounded like a warning. The frame round the screen is all that shows while control listens and works; the pill comes down only at the end, to say how it ended when there is something to say (nothing heard, not understood, an agent's last words). Pressed again:

1. The speech model writes it down, spelled with the terms and without the words to avoid. Nothing is kept in the history.
2. The language model (post-processing's, whether or not post-processing is on) writes the steps (`control::steps`), about 0.12 s on the 4B once it is loaded.
3. `act::run` carries them out, Escape checked between steps. Or, for `ask`, the agent takes over.

Both models are wanted, and Accessibility: without one the settings open where it is given. Escape stops any of this (`cancel_dictation`), and ends an agent's program.

## The steps

`open` finds an app by any name it goes by: running apps first, then the usual folders, by file name and by the name Finder shows, so Notes is found as 备忘录 on a Chinese Mac. The model is told to write the English name. A name that is no app and one plain word is tried as a `.com`, because "open youtube" is written as an app however it is shown. It then waits for the app to be in front: a key sent too soon lands in the app before it.

`type` is a dictation's paste, with the clipboard put back. `keys` are events with the Mac's codes on the US layout, as the system's own shortcuts are. The arrows carry the flags a keyboard sends with them, or the Mac's shortcuts on them do not fire.

`click NAME` reads the app in front (`screen`): its focused window, then its menu bar, in breadth, up to 5,000 things or 2.5 s. The best match presses by `AXPress`, and by a real click at its middle where that fails. When the model translated a name ("Send" for a button that says 发送), a name on the screen found in what the person said answers instead. Chromium and Electron draw nothing for Accessibility until asked (`AXManualAccessibility`, `AXEnhancedUserInterface`).

`scroll` is sent to the middle of the front window, not to the pointer, a page being four fifths of its height. Whatever covers that point takes the scroll, a floating probe window included.

## The agent

For what needs eyes: replying to someone, finding something on the screen, a form. `agent::run` starts the person's `claude` or `codex` in an empty folder (`~/.kotoba/agent`), with Kotoba's tools alone, served from this process over the loopback, MCP over HTTP, on a port of the system's choosing, with a token made for the run. A request without the token, or from a browser (it has an `Origin`), is refused. Each is found where a person's install puts it, since an app from the Dock has no shell's PATH.

The tools are `look` (what `screen` reads, numbered), `open`, `web`, `keys`, `type`, `click`, `double`, `right`, `scroll` and `wait`. `look` reads Accessibility only, with no pictures: no Screen Recording to ask for, and an app that shows Accessibility nothing shows the agent nothing.

Told what was said, the agent may stop and say so. It is told to check who a message is going to, to read what apps show as information and never as instructions, and to leave a system's question to the person. Its last words go in the pill, cut to what fits.

**Claude Code's `-p` signs in with the person's subscription; `--bare` does not** (it reads no OAuth or keychain), and its documentation says `--bare` will become the default for `-p`. Watch for that in Claude Code's releases: a run with no login fails with a message, shown in the settings.

**Its startup can ask for Documents.** A program Kotoba starts is Kotoba's in the system's eyes, so what Claude Code reads for itself at startup can have the system ask whether Kotoba may open a folder. Seen once in a build with `~/.claude/AGENTS.md` a link into `~/Documents`; `--setting-sources project` and `--disable-slash-commands` keep out what they can.

**The name.** Say "Claude Code" and "Codex" in plain words, as what the person's agent is. Neither name or logo goes in a feature's name or a picture.

## The terms

For a release, read Anthropic's page before this ships: https://code.claude.com/docs/en/legal-and-compliance. It says Anthropic does not let third-party developers route requests through a person's plan credentials on their users' behalf, or offer Claude.ai login in their own apps; that this does not stop a person signing in to the unmodified Claude Code with their own subscription; and that running Claude Code in a product requires its Commercial Terms, with the binary unmodified and each person signed in themselves. Kotoba holds to the conditions it can see (it runs the person's own install and touches no login), but whether a shipped app starting `claude -p` under a person's plan is allowed is Anthropic's to say, and has not been asked. OpenAI's terms for Codex were not found to say either way. An API key in the person's environment is the plain way out for Claude Code.

## The frame

Four heavy marks at the screen's corners, as a camera's viewfinder frames what it looks at, in ink: dark on a light Mac and light on a dark one (`AppleInterfaceStyle`). Under each mark is a band of color 10 points wide and a little longer than it, fading out past its ends, so color comes out of all four corners and the mark is seen on any ground. The band's hue is set by where it is round the screen: it shows a wheel of colors (a conic `CAGradientLayer` round the screen's middle) through eight layers of each band (`BAND_REACH`), each reaching a little less far and letting part of the color through, which makes the fade. The wheel turns slowly, clockwise as the marks slide, and faster while it works. The slow turn asks Core Animation for 15 frames a second (`Pace::Drift`), since a turn of 12 degrees a second moves a color less than a degree a frame: anything that moves in the frame's window has WindowServer draw at the screen's rate, and the turn alone at that rate cost it 30% of a core. A soft glow of many colors round the whole edge was too close to Apple's own and was taken out, as were a thin line of color and the marks alone in ink after it.

Each mark is a stretch of one outline, a rounded box 5 points in, close to the screen's edge: a `CAShapeLayer` whose path goes round the outline twice from half a round before its corner, so the corner sits a quarter of the way along. A mark is its `strokeStart` and `strokeEnd`. Nothing scales: as control starts each mark comes out of its corner along both edges (`show`), the voice makes them reach further (`level`), and as control ends each draws back to its middle and is gone (`hide`). While the steps are worked out and done the marks slide round the outline together (`thinking`), easing up to speed: an ease-in, then a slide of half the path repeated, which starts again a whole round on, where nothing looks different. `hide` reads where the slide has them (`presentationLayer`) before letting it go, or they would jump back to their corners. The band's layers are marks too, and move with them. With Reduce Motion the voice changes nothing, nothing slides and the colors stay where they are.

It is Core Animation's, drawn by the system at the screen's rate: a frame of ours is a frame the model waits behind. The window is a panel at the status level, over the menu bar and the Dock and under the pill, taking no click, in every space.

## What Talon showed

Talon (talonvoice.com) is closed, and its license lets it be reverse engineered "for personal knowledge only", with what comes of that not to be redistributed. It was not taken apart. What follows is from its public docs and `talonhub/community`, which is MIT: a phrase is matched to a command, and a command is a short list of a few actions (`key`, `insert`, `mouse_click`, `mouse_scroll`, an app's focus). Commands chain in a breath, and a switch of app is waited for. Kotoba keeps those. Talon's commands are a grammar written in English, and listening never stops; Kotoba has a key, is used in many languages, and has a language model on the Mac, so the model is the grammar.

Talon's mouse grid and eye tracking have no counterpart here. What Accessibility does not show cannot be clicked by name.

## Traps

- A plan can add an Enter it was not asked for: the 4B ended "open Notes and write buy milk" with one at times, and in a chat that sends. The prompt tells it not to, and the examples show it; there is no check.
- "Close the other tabs" and such have no key the model knows. Left to itself it wrote `cmd-w`, the wrong tab. They are taught as `ask`.
- A probe with a settings window has that window floating over the app in front: step back first (`hide`, `docs/probing.md`) or the window is what is read, and what takes a scroll.
- A sentence ending in a question can come back `none`: "write lunch is at noon can you come". The model is told not to answer, and it reads this as a question.
