//! The window's faces. Words are in the system's face, as Emaki's chrome
//! is. The app's name, on About, is in a light, elegant sans with flared
//! strokes: Optima, else Avenir Next, both the Mac's own and never
//! carried; with neither, Belleza (etunni/belleza), a free face of the
//! same kind under the SIL Open Font License, carried with its license
//! beside it in `assets/fonts`.

use gpui::prelude::FluentBuilder as _;
use gpui::{div, px, App, Global, ParentElement as _, SharedString, Styled as _};

/// The faces the name is tried in, most wanted first.
const WORDMARK_FACES: &[&str] = &["Optima", "Avenir Next", "Avenir", "Belleza"];

/// The faces 言葉 is set in beside the name: a Mincho, the Mac's own and
/// never carried.
const KANJI_FACES: &[&str] = &["Hiragino Mincho ProN", "Hiragino Mincho Pro"];

/// 言葉, to stand after the name set at `size` on About: in a Mincho, at
/// 0.78 of the name, the two a little apart and a little off it. gpui has
/// no letter spacing, so each character is drawn on its own.
pub fn kanji(size: f32, cx: &App) -> gpui::Div {
    let at = size * 0.78;
    let face = cx.global::<Faces>().kanji.clone();
    div().flex().flex_shrink_0().ml(px(size * 0.18)).gap(px(at * 0.06)).text_size(px(at)).when_some(face, |d, f| d.font_family(f)).child("言").child("葉")
}

/// What the system has of the faces wanted, found once at launch: listing
/// every installed face takes about a tenth of a second, and asked as the
/// window draws it held every frame that long.
struct Faces {
    wordmark: Option<SharedString>,
    kanji: Option<SharedString>,
}

impl Global for Faces {}

/// The carried face handed to the text system, and the faces wanted
/// found among what the system has.
pub fn install(cx: &mut App) {
    let _ = cx.text_system().add_fonts(vec![std::borrow::Cow::Borrowed(include_bytes!("../assets/fonts/Belleza-Regular.ttf").as_slice())]);
    let names = cx.text_system().all_font_names();
    let first = |faces: &[&'static str]| faces.iter().find(|f| names.iter().any(|n| n == *f)).map(|f| SharedString::from(*f));
    let faces = Faces { wordmark: first(WORDMARK_FACES), kanji: first(KANJI_FACES) };
    cx.set_global(faces);
}

/// The family the app's name is set in, or None to leave the window's
/// own face.
pub fn wordmark_family(cx: &App) -> Option<SharedString> {
    cx.global::<Faces>().wordmark.clone()
}
