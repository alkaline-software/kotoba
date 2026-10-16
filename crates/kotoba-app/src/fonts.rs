//! The window's faces. Words are in the system's face, as Emaki's chrome
//! is. The app's name, on About, is in a light, elegant sans with flared
//! strokes: Optima, else Avenir Next, both the Mac's own and never
//! carried; with neither, Belleza (etunni/belleza), a free face of the
//! same kind under the SIL Open Font License, carried with its licence
//! beside it in `assets/fonts`.

use gpui::{App, Global, SharedString};

/// The faces the name is tried in, most wanted first.
const WORDMARK_FACES: &[&str] = &["Optima", "Avenir Next", "Avenir", "Belleza"];

struct Wordmark(Option<String>);

impl Global for Wordmark {}

/// The carried face handed to the text system, and the name's face
/// found among what the system has.
pub fn install(cx: &mut App) {
    let _ = cx.text_system().add_fonts(vec![std::borrow::Cow::Borrowed(include_bytes!("../assets/fonts/Belleza-Regular.ttf").as_slice())]);
    let names = cx.text_system().all_font_names();
    let wordmark = WORDMARK_FACES.iter().find(|f| names.iter().any(|n| n == *f)).map(|f| f.to_string());
    cx.set_global(Wordmark(wordmark));
}

/// The family the app's name is set in, or None to leave the window's
/// own face.
pub fn wordmark_family(cx: &App) -> Option<SharedString> {
    cx.global::<Wordmark>().0.clone().map(SharedString::from)
}
