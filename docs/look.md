# Look

About `crates/kotoba-app/src/look.rs`, `fonts.rs`, `assets.rs`, `ui.rs`,
`assets/`, `themes/kotoba.json`, and the app icon.

## Rules

- A button is `ui::choice_pill` (or `ui::small_pill` in a row): a pill
  with an icon before its word. The toolkit's `Button` is not used for
  one. `Press` says how it is drawn: quiet, the danger's for what cannot
  be taken back, filled with the accent for what the page is for.
- Every key the window shows is drawn by `ui::key_text`, on caps; a
  hover's label goes through `ui::tip`, which does the same.
- No call site names Tabler. A new icon is a file in `assets/icons/`
  under the toolkit's file name and a line in `OWN` (`assets.rs`): a file
  not named there draws nothing, and nothing says so.
- Call `look::apply`, never `sync_system_appearance`, or a pinned
  appearance flips with the system.
- Paint the accent into both theme configs before the toolkit gets them
  (`look::install`), so `Theme::change` keeps it on every switch.
- Write settings through `Config::edit`, which writes a temp file and
  renames it.
- A clip in gpui is a rectangle: a ground that reaches a rounded card's
  corner is rounded there itself.

## The palette

Emaki's (`themes/kotoba.json`): near white with a trace of warmth in the
light appearance, the darkest ground the page in the dark one. Kotoba's
accent is green, as Emaki's is terracotta: `#519F5C`, the same chroma as
terracotta in OKLCH, with a darker hover and press in the light
appearance and a lighter hover in the dark. It is the only accent
(`look::GREEN`): a menu bar app's one window has no need of a choice,
and the setting Emaki has was taken out.

## Icons

Tabler's outline icons, as Emaki's, each under the name the toolkit or
the app asks for, with `TABLER-LICENSE` beside them. The models' makers
are LobeHub's marks (Qwen's, and OpenAI's for Whisper), with
`makers/LOBEHUB-LICENSE`.

## The app icon

`scripts/icon/icon.html` is the source, drawn on a canvas in Emaki's
icon's manner: the Alkaline Software organisation's cream ground, navy
lines, an amber stroke and the violet spark, with green in place of
terracotta. The subject is a leaf (葉, the second character of 言葉) that
is also a speech bubble: tilted up to the right, its outline built from
two arcs, its tail at the lower left a short point whose lower edge
curves back into the leaf, and the leaf's tip at the upper right
rounded by a small arc. Its veins are six upright sound bars of the
first drawing's lengths, the fourth amber. Upright bars land on whole
pixels, so 32 px stays crisp; below 24 px across only the three middle
bars are drawn, navy, amber, navy. It was picked from three rounds of
candidates (`VARIANT` in the page; 15 is the one in use), after a
symmetric upright leaf read as too stiff and a 45 degree one with
slanted veins as too far from the first drawing.
`scripts/make-icon.sh` renders every size on its own pixels
(`render.swift`, a windowless `WKWebView`), the `.icns` as the full
opaque square and the PNGs as the rounded icon with its margin, and the
menu bar's template image from the same page: the bubble leaf in black
with four bars cut out.

## The faces

Words are the system's face, as Emaki's chrome is. The app's name on
About is in Optima, else Avenir Next (the Mac's own, never carried),
else Belleza, carried under the SIL Open Font License (`fonts.rs`).
