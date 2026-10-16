# Design

The standard every part of Kotoba a person sees is held to: the
settings window, its sheets and lists, the pill, the menu bar item's
menu. The bar is the work that wins Awwwards, the Webbys and the FWA:
minimal, calm, intuitive, finished in every state. What the palette,
icons and faces are is in `docs/look.md`; how the window works is in
`docs/window.md`.

## Rules

- **Calm.** One accent, green (`look::GREEN`), for what is on, in use
  or the one thing a page is for. Nothing else competes with it: a
  page with many of the same action (seven Downloads) draws them quiet.
- **Everything fits.** Every section shows all it has without
  scrolling. The page still scrolls, for what a fold opens or what is
  added later; a new row that would push a section past the window
  means the section is reworked, not left to scroll.
- **Nothing jumps.** What appears, goes, opens, closes, grows, moves,
  fills or turns does so over a moment, with the pattern its neighbours
  use (`docs/window.md`, Motion). A section's name fades with its page.
- **Words are for everyone.** Kotoba is used in many languages, spoken
  and written: nothing says how it does in one of them (a model's line
  says what it is for all, never "weaker Chinese").
- **Words are few.** A row's name says what it is. A line under it is
  there only where the name does not say it all, in a few plain words,
  and never repeats what the control shows.
- **One way to draw each thing.** A setting is a row on a card
  (`line`, `group`); on or off is a switch; a choice among a few is
  `segmented`; a list of many drops down (`settings_pick_h`); a button
  is a pill with an icon (`ui::small_pill`, `ui::choice_pill`). A new
  control is a new pattern for every page, or it is not made.
- **Things line up.** Names start on one line, controls end on
  another, and the widest controls on a page start and end together.
  Every field, button and choice in a row is one height.
- **Every language.** A label never depends on English being short:
  controls grow to their words, and a page that fits in English fits in
  French and German. Each language is written as its own Mac writes it,
  in its own punctuation and with the system's names for its settings.
- **Both appearances, every state.** Light and dark are designed
  together, never one inverted. A change is looked at in both, and in
  every state it has: empty, full, refused, waiting, off, mid-motion.

## The window

820 by 664. The rail at the left is 190 wide; the page has a margin of
36 at either side. The section's name stands in the window's top (52
tall), level with the traffic lights, 15 points semibold, as the Mac's
own settings have it.

## Groups and rows

A section is groups, 18 apart. A group is its name, 12 points
semibold in the quiet ink, 6 over its card, in line with the rows'
names (14 in from the card's edge); what belongs to the whole group (a
lane's switch) stands at the name's right, in line with the controls.

A card is rounded 12, with a hairline edge in the border's colour and a
ground a shade off the page: the muted colour at 40% in the light
appearance, the sidebar's in the dark (`card_ground`). Its rows are
parted by the same hairline. A row is at least 46 tall, 14 in at either
side, 8 above and below: its name in 13 points, its line under it in
11.5 on 15 in the quiet ink, and its control at the right. What a row
says of itself later (a key not taken, Fn's guide) folds open under it,
across the card.

A row that paints its own ground (the model in use, washed with the
accent) rounds it where it meets the card's corners: a clip in gpui is
a rectangle.

## Controls

- Every field, button and choice in a row is 28 tall. Text in a control
  is 12.5 points.
- A switch is 36 by 20, its knob 16, the track the switch colour when
  off and the accent when on.
- The widest controls are 336 (`CONTROL_W`): a field of 160 and two
  pills of 80, or a field and one pill filling the rest. A permission's
  state and the update's button have widths of their own, set once.
- A control on a card takes the muted shade, never the sidebar's (the
  dark card is the sidebar's shade).
- `Press::Quiet` is the default. `Press::Primary`, filled with the
  accent, is for the one thing a page is waiting on (Allow).
  `Press::Danger` is for what cannot be taken back (Remove, Delete
  All), which asks once more where much is lost.

## The menu bar item's menu

The system's own menu, never one drawn by the app: the sections in the
rail's order with the rail's icons, a line between the settings and
Updates and About, and another before Quit Kotoba. Its icons are the
window's Tabler icons as template images, so the system tints them.

## Checking

Shoot every state a change touches, in both appearances
(`docs/probing.md`), and look at it whole and close up. Then go round
again: what is out of line, louder than it needs to be, or not
consistent with its neighbours is fixed and shot again, until nothing
is left to change.
