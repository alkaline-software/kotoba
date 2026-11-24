# Design

The standard every part of Kotoba a person sees is held to: the settings window, its sheets and lists, the pill, the menu bar item's menu. The bar is the work that wins Awwwards, the Webbys and the FWA: minimal, calm, intuitive, finished in every state. What the palette, icons and faces are is in `docs/look.md`; how the window works is in `docs/window.md`.

## Rules

- **Calm.** One accent, green (`look::GREEN`), for what is on, in use or the one thing a page is for. Nothing else competes with it: a page with many of the same action (seven Downloads) draws them quiet. A color that stands for something else (a model maker's mark) marks that thing and nothing else. Control's frame round the screen is the one place many colors show, under its dark marks, since it sits over every app and stands for none (`docs/control.md`, The frame).
- **Faded means out of reach.** Only what does not answer a press is drawn faded. What is off but can still be used stays at full ink; off shows in its switch and in the accent going.
- **Everything fits.** Every section shows all it has without scrolling. The page still scrolls, for what a fold opens or what is added later; a new row that would push a section past the window means the section is reworked, not left to scroll. Two sections that read as one are merged: the updates are under About.
- **Nothing jumps.** What appears, goes, opens, closes, grows, moves, fills or turns does so over a moment, with the pattern its neighbors use (`docs/window.md`, Motion). A section's name fades with its page.
- **No pause at the end of a motion.** It runs into its place and stops, with no crawl, hesitation or second step in its last frames. Every new animation has had one until it was looked for, so look for it.
- **What is cut says so.** Content that runs past an edge fades out at that edge, and can be reached.
- **A gesture stays where it began.** A scroll, its momentum included, moves only the scroller it started in and stops at its edge (`docs/window.md`, Scroll routing).
- **Words are for everyone.** Kotoba is used in many languages, spoken and written: nothing says how it does in one of them (a model's line says what it is for all, never "weaker Chinese").
- **Words are few.** A row's name says what it is. A line under it is there only where the name does not say it all, in a few plain words, and never repeats what the control shows.
- **One way to draw each thing.** A setting is a row on a card (`line`, `group`); on or off is a switch; a choice among a few is `segmented`; a list of many drops down (`settings_pick_h`); a button is a pill with an icon (`widgets::small_pill`, `widgets::choice_pill`); a key is a cap. A new control is a new pattern for every page, or it is not made.
- **Things line up.** Names start on one line, controls end on another, and the widest controls on a page start and end together. Every field, button and choice in a row is one height.
- **Every language.** A label never depends on English being short: controls grow to their words, and a page that fits in English fits in French and German. Each language is written as its own Mac writes it, in its own punctuation and with the system's names for its settings.
- **Both appearances, every state.** Light and dark are designed together, never one inverted: every color has a value in each. A change is looked at in both, and in every state it has: empty, full, refused, waiting, off, hover, pressed, mid-motion, the window at its smallest (860 by 460).
- **Readable.** Words read on their ground at 4.5 to 1 or better, a control's edge at 3 to 1. A key is drawn as a key, never as code.
- **Look before saying it is done.** At all the change shows, hides or moves, in both appearances and in the window at its smallest (Checking).

## The window

860 by 600, as every product of the studio's. The rail at the left is 190 wide, its rows starting under the traffic lights, 30 tall each, an icon and a name, the chosen one on a plate; it stands on the muted shade at half in the light appearance and the sidebar's in the dark. A section with something waiting (a permission not given, a newer Kotoba) wears a dot in the accent on its rail row. The page has a margin of 36 at either side. The section's name stands in the window's top (52 tall), level with the traffic lights, 15 points semibold, as the Mac's own settings have it.

## Groups and rows

A section is groups, 18 apart. A group is its name, 12 points semibold in the quiet ink, 6 over its card, in line with the rows' names (14 in from the card's edge); what belongs to the whole group (a lane's switch) stands at the name's right, in line with the controls.

A card is rounded 12, with a hairline edge in the border's color and a ground a shade off the page: the muted color at 40% in the light appearance, the sidebar's in the dark (`card_ground`). Its rows are parted by the same hairline. A row is at least 46 tall, 14 in at either side, 8 above and below: its name in 13 points, its line under it in 11.5 on 15 in the quiet ink, and its control at the right. What a row says of itself later (a key not taken, Fn's guide) folds open under it, across the card.

A row that paints its own ground (the model in use, washed with the accent) rounds it where it meets the card's corners: a clip in gpui is a rectangle.

## Controls

- Every field, button and choice in a row is 28 tall. Text in a control is 12.5 points.
- A switch is 36 by 20, its knob 16, the track the switch color when off and the accent when on.
- The widest controls are 336 (`CONTROL_W`): a field of 160 and two pills of 80, or a field and one pill filling the rest. A permission's state and the update's button have widths of their own, set once.
- A control on a card takes the muted shade, never the sidebar's (the dark card is the sidebar's shade).
- `Press::Quiet` is the default. `Press::Primary`, filled with the accent, is for the one thing a page is waiting on (Allow, Update to x). `Press::Danger` is for what cannot be taken back (Remove, Delete All), which asks once more where much is lost.

## About

The last section is the studio's own, the same in every product: a card holding the title card, set as a film's first one is: "Alkaline Software presents" at 14 points, the app's icon, its name at 42 in the name's face and the accent with 言葉 beside it in a Mincho at 0.78 of its size, as a Japanese name has (`fonts::kanji`), and the version in small gray words. Under the card, pills to the source and the release's notes; Kotoba has no site, so no pill to one. Then the updates: a card of two rows, Updates (what a check found, and the one button that checks or installs) and Check automatically, with no line under it. The studio is the only name it gives. No gradients.

## The menu bar item's menu

The system's own menu, never one drawn by the app: the sections in the rail's order with the rail's icons, a line before About, and another before Quit Kotoba. Its icons are the window's Tabler icons as template images, so the system tints them.

## The README's picture

The README opens with one moving picture (`.github/readme.gif`): what Kotoba does, played as a person does it, drawn in the hand of the studio's sites and never recorded from the screen. A recording carries one machine's fonts and whatever was open, and goes stale with every change of the window; a drawing is made again in seconds. Kotoba has no picture yet. The first is drawn as below and put at the head of the README as `<p align="center"><img src=".github/readme.gif" alt="…" width="100%"></p>`, its alt saying in a sentence or two what plays.

- **The hand.** Hairlines on warm paper (`#F3F1EA`), a window a sheet of `#F8F6F0` with an edge of `#8E8A80`, words in `#1F1E1D` and `#77746B`; each part a pale wash of its color (9%) with an edge of it, its name in its color; lines stand for words. Source Serif 4 for the name, the title and what a part is called, its italic for what something does, JetBrains Mono for a key or a short label, all from Google Fonts so every machine renders the same picture.
- **The layout.** In a 1920 by 1008 box: the icon and the name (with its kanji, where it has one) at the top left, the system at the top right in the accent, and the title under them, one line of what Kotoba is for. The stage at the left; at the right a list of what plays, a row each with its keys on caps, lit in its color while it plays and a shade darker once it has.
- **The story** is what Kotoba has now. Each move is what a person does (a click on what does it, a drag, a key), with the pointer the Mac draws, on the curves the window uses for it. For the loop the stage fades out where it ended and in as it began, so the story never runs backward.

`scripts/readme/readme.html` is the drawing: a canvas page that plays in a browser and draws any moment on its own. It keeps a script of times and works every place out from it, and exposes `ready`, `FRAMES` and `frame(i)`, which returns frame i as a PNG; nothing else in the page is named `frame`, or the two call each other. `scripts/make-readme-gif.sh` renders it on a Mac with ffmpeg: `scripts/readme/render.swift` loads the page in a windowless WebKit view and writes every frame, the icon handed in as data (`README_ICON`: drawn from a file it taints the canvas, and no frame can be read back), and ffmpeg makes the GIF with one palette for the whole loop, at 30 frames a second, 12 to 15 seconds and about 3 MB at most. `scripts/make-readme-gif.sh <dir> <first> <last>` writes frames as PNGs to look at first: a sheet of every 45th frame, each move's middle, and the loop's last and first frames side by side. Two things fading in one place stand over each other in a smudge, so one goes before the other comes.

A release that changes what the picture shows (a part, its place, its keys, how it is brought in) changes the drawing in the same pass, and the GIF is rendered again and committed with it.

## Checking

Shoot every state a change touches, in both appearances (`docs/probing.md`), and in French or German, and look at it whole and close up. Then go round again: what is out of line, louder than it needs to be, or not consistent with its neighbors is fixed and shot again, until nothing is left to change.
