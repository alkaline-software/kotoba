# Look

About `crates/kotoba-app/src/look.rs`, `fonts.rs`, `assets.rs`, `widgets.rs`, `assets/`, `themes/kotoba.json`, and the app icon.

## Rules

- A button is `widgets::choice_pill` (or `widgets::small_pill` in a row): a pill with an icon before its word. The toolkit's `Button` is not used for one. `Press` says how it is drawn: quiet, the danger's for what cannot be taken back, filled with the accent for what the page is for.
- Every key the window shows is drawn by `widgets::key_text`, on caps; a hover's label goes through `widgets::tip`, which does the same.
- No call site names Tabler. A new icon is a file in `assets/icons/` under the toolkit's file name; the folder is built in whole (`assets.rs`, by rust-embed), so there is no list to keep. A picture named in the code and not carried draws nothing, or the toolkit's own in its place, and nothing says so: a test fails on one (`every_named_picture_is_carried`).
- Call `look::apply`, never `sync_system_appearance`, or a pinned appearance flips with the system.
- Paint the accent into both theme configs before the toolkit gets them (`look::install`), so `Theme::change` keeps it on every switch.
- Write settings through `Config::edit`, which writes a temp file and renames it.
- A clip in gpui is a rectangle: a ground that reaches a rounded card's corner is rounded there itself.
- A color that is not in the theme file or `look.rs` is a mistake.

## The palette

The studio's neutrals (`themes/kotoba.json`): near white with a trace of warmth in the light appearance, the darkest ground the page in the dark one. The file carries the accent too, in the same shades `look.rs` paints, so the file alone says what is drawn.

The accent is green, the leaf's hue (147 in OKLCH) at the studio's lightness for an accent: 0.52 in the light appearance and 0.74 in the dark, its chroma eased to 0.084 as the studio's greens are, since a green at the full chroma is loud. White is drawn on the light shade (5.3 to 1) and the ink on the dark one (7.4 to 1). It is the only accent (`look::GREEN`): a menu bar app's one window has no need of a choice. Tried and dropped: a lighter green, `#519F5C` in both appearances, on which white read at 3.3 to 1.

## Icons

Tabler's outline icons at a stroke of 1.75, each under the name the toolkit or the app asks for, with `TABLER-LICENSE` beside them. The models' makers are LobeHub's marks (Qwen's, and OpenAI's for Whisper), with `makers/LOBEHUB-LICENSE`.

## The app icon

`scripts/icon/icon.html` is the source, drawn on a canvas in the studio's manner: navy lines, an amber stroke and the violet spark, with the leaf's green as the product's own color, on the Alkaline Software cream with a fifth of the accent in it (`ground()` on the page: each end of the cream mixed with the mean of the accent's two base shades, 16% at the top left and 24% at the bottom right, `#e5e9d9` to `#d0d2b8`), so the Dock tells the studio's icons apart by their grounds while the drawings stay one hand. The subject is a leaf (葉, the second character of 言葉) that is also a speech bubble: tilted up to the right, its outline built from two arcs, its tail at the lower left a short point whose lower edge curves back into the leaf, and the leaf's tip at the upper right rounded by a small arc. Its veins are six upright sound bars of the first drawing's lengths, the fourth amber. Upright bars land on whole pixels, so 32 px stays crisp; below 24 px across only the three middle bars are drawn, navy, amber, navy. It was picked from three rounds of candidates (`VARIANT` in the page; 15 is the one in use), after a symmetric upright leaf read as too stiff and a 45 degree one with slanted veins as too far from the first drawing.

`scripts/make-icon.sh` renders every size on its own pixels (`render.swift`, a windowless `WKWebView`) into `crates/kotoba-app/assets/icon`: the `.icns` as the full opaque square, since macOS 26 masks every app icon to its own rounded square and shows anything transparent as a gray border; the PNGs, for the window, as the rounded icon with its margin and shadow. The menu bar's template image comes from the same page, the bubble leaf in black with four bars cut out, which the system tints: `assets/icons/menubar.png` at 36 px (18 pt at 2x) and `menubar-1x.png` at 18. Open the page in a browser to see every size on one sheet. Change the drawing, run the script, rebuild, commit the assets.

## The faces

Words are the system's face. The app's name on About is in Optima, else Avenir Next (the Mac's own, never carried), else Belleza, carried under the SIL Open Font License (`fonts.rs`).
