//! The pictures the window draws, over the set gpui-component ships.
//!
//! `Icon::new(IconName::…)` and `Icon::default().path(…)` resolve through
//! the app's `AssetSource`, and this one answers first for every file it
//! carries, then hands the rest to the toolkit's own set:
//!
//! - `icons/`: Tabler's outline icons (MIT, `TABLER-LICENSE` beside them),
//!   each under the name the toolkit or the app asks for (`close.svg` is
//!   Tabler's `x`), so the icons the toolkit draws by itself are Tabler's
//!   too; and the models' makers' marks, LobeHub's (MIT, `makers/`).
//! - `icon/`: the app's own icon at each size `scripts/make-icon.sh` writes.
//!
//! The folders are built in whole, so a file put there is served: no list
//! to keep beside them.

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

#[derive(rust_embed::RustEmbed)]
#[folder = "assets"]
#[include = "icons/**"]
#[include = "icon/*.png"]
pub struct Own;

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(f) = Own::get(path) {
            return Ok(Some(f.data));
        }
        gpui_component_assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut v = gpui_component_assets::Assets.list(path)?;
        v.extend(Own::iter().filter(|p| p.starts_with(path)).map(|p| SharedString::from(p.into_owned())));
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every picture the window names by path is carried: one that is not
    /// draws nothing, or the toolkit's own in its place, and nothing says
    /// so.
    #[test]
    fn every_named_picture_is_carried() {
        let sources = [
            include_str!("main.rs"),
            include_str!("kotoba.rs"),
            include_str!("settings.rs"),
            include_str!("controls.rs"),
            include_str!("widgets.rs"),
            include_str!("sys.rs"),
            include_str!("pill.rs"),
        ];
        let mut missing = Vec::new();
        for src in sources {
            for (at, _) in src.match_indices("\"icon") {
                let rest = &src[at + 1..];
                let path = &rest[..rest.find('"').unwrap_or(0)];
                if (path.starts_with("icons/") || path.starts_with("icon/")) && (path.ends_with(".svg") || path.ends_with(".png")) && Own::get(path).is_none() {
                    missing.push(path.to_string());
                }
            }
        }
        assert!(missing.is_empty(), "not in assets/: {missing:?}");
    }
}
