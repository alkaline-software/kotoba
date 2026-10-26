# Design

The standard every part of Kotoba a person sees is held to: the settings window, its sheets and lists, the pill, the menu bar item's menu. The bar is the work that wins Awwwards, the Webbys and the FWA: minimal, calm, intuitive, finished in every state. What the palette, icons and faces are is in `docs/look.md`; how the window works is in `docs/window.md`.

## Rules

- **Calm.** One accent, green (`look::GREEN`), for what is on, in use or the one thing a page is for. Nothing else competes with it: a page with many of the same action (seven Downloads) draws them quiet. A color that stands for something else (a model maker's mark) marks that thing and nothing else.
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
- **Both appearances, every state.** Light and dark are designed together, never one inverted. A change is looked at in both, and in every state it has: empty, full, refused, waiting, off, hover, pressed, mid-motion.
- **Readable.** Words read on their ground at 4.5 to 1 or better, a control's edge at 3 to 1. A key is drawn as a key, never as code.
- **Look before saying it is done.** At all the change shows, hides or moves, in both appearances (Checking).

## The window

820 by 664. The rail at the left is 190 wide, its rows starting under the traffic lights, 30 tall each, an icon and a name, the chosen one on a plate; it stands on the muted shade at half in the light appearance and the sidebar's in the dark. A section with something waiting (a permission not given, a newer Kotoba) wears a dot in the accent on its rail row. The page has a margin of 36 at either side. The section's name stands in the window's top (52 tall), level with the traffic lights, 15 points semibold, as the Mac's own settings have it.

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

The last section is a title card, set as a film's first one is: "Alkaline Software presents", the app's icon, its name in the name's face and the accent, 言葉 and the version in small gray words. Under it a sentence naming the studio's owners, each name a link, and pills to the source and the release's notes. Under the credits, the updates: a card of two rows, Updates (what a check found, and the one button that checks or installs) and Check automatically. No gradients.

## The menu bar item's menu

The system's own menu, never one drawn by the app: the sections in the rail's order with the rail's icons, a line before About, and another before Quit Kotoba. Its icons are the window's Tabler icons as template images, so the system tints them.

## Checking

Shoot every state a change touches, in both appearances (`docs/probing.md`), and in French or German, and look at it whole and close up. Then go round again: what is out of line, louder than it needs to be, or not consistent with its neighbors is fixed and shot again, until nothing is left to change.
