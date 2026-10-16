//! Our own icons, over the set gpui-component ships.
//!
//! `Icon::new(IconName::…)` resolves through the app's `AssetSource`; this
//! wrapper answers for the files under `assets/icons/` and hands everything
//! else to the bundled set. The files are Tabler's outline icons (MIT,
//! `TABLER-LICENSE` beside them), as Emaki's are, each under the name the
//! toolkit or the app asks for (`close.svg` is Tabler's `x`); the models'
//! makers' marks are LobeHub's (MIT). A file not named in `OWN` draws
//! nothing, and nothing says so.

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

pub struct Assets;

const OWN: &[(&str, &[u8])] = &[
    ("icon/app-large.png", include_bytes!("../assets/icon/icon-512.png")),
    ("icons/arrow-down.svg", include_bytes!("../assets/icons/arrow-down.svg")),
    ("icons/arrow-up.svg", include_bytes!("../assets/icons/arrow-up.svg")),
    ("icons/ban.svg", include_bytes!("../assets/icons/ban.svg")),
    ("icons/case-sensitive.svg", include_bytes!("../assets/icons/case-sensitive.svg")),
    ("icons/check.svg", include_bytes!("../assets/icons/check.svg")),
    ("icons/chevron-down.svg", include_bytes!("../assets/icons/chevron-down.svg")),
    ("icons/chevron-up.svg", include_bytes!("../assets/icons/chevron-up.svg")),
    ("icons/chevrons-up-down.svg", include_bytes!("../assets/icons/chevrons-up-down.svg")),
    ("icons/close.svg", include_bytes!("../assets/icons/close.svg")),
    ("icons/copy.svg", include_bytes!("../assets/icons/copy.svg")),
    ("icons/cpu.svg", include_bytes!("../assets/icons/cpu.svg")),
    ("icons/device-desktop.svg", include_bytes!("../assets/icons/device-desktop.svg")),
    ("icons/download.svg", include_bytes!("../assets/icons/download.svg")),
    ("icons/external-link.svg", include_bytes!("../assets/icons/external-link.svg")),
    ("icons/file-text.svg", include_bytes!("../assets/icons/file-text.svg")),
    ("icons/github.svg", include_bytes!("../assets/icons/github.svg")),
    ("icons/globe.svg", include_bytes!("../assets/icons/globe.svg")),
    ("icons/history.svg", include_bytes!("../assets/icons/history.svg")),
    ("icons/info.svg", include_bytes!("../assets/icons/info.svg")),
    ("icons/key-command.svg", include_bytes!("../assets/icons/key-command.svg")),
    ("icons/key-option.svg", include_bytes!("../assets/icons/key-option.svg")),
    ("icons/key-return.svg", include_bytes!("../assets/icons/key-return.svg")),
    ("icons/key-shift.svg", include_bytes!("../assets/icons/key-shift.svg")),
    ("icons/key-tab.svg", include_bytes!("../assets/icons/key-tab.svg")),
    ("icons/list-bullets.svg", include_bytes!("../assets/icons/list-bullets.svg")),
    ("icons/logout.svg", include_bytes!("../assets/icons/logout.svg")),
    ("icons/makers/openai.svg", include_bytes!("../assets/icons/makers/openai.svg")),
    ("icons/makers/qwen.svg", include_bytes!("../assets/icons/makers/qwen.svg")),
    ("icons/microphone.svg", include_bytes!("../assets/icons/microphone.svg")),
    ("icons/moon.svg", include_bytes!("../assets/icons/moon.svg")),
    ("icons/palette.svg", include_bytes!("../assets/icons/palette.svg")),
    ("icons/pencil.svg", include_bytes!("../assets/icons/pencil.svg")),
    ("icons/plus.svg", include_bytes!("../assets/icons/plus.svg")),
    ("icons/redo-2.svg", include_bytes!("../assets/icons/redo-2.svg")),
    ("icons/rotate.svg", include_bytes!("../assets/icons/rotate.svg")),
    ("icons/settings.svg", include_bytes!("../assets/icons/settings.svg")),
    ("icons/shield.svg", include_bytes!("../assets/icons/shield.svg")),
    ("icons/sparkles.svg", include_bytes!("../assets/icons/sparkles.svg")),
    ("icons/sun.svg", include_bytes!("../assets/icons/sun.svg")),
    ("icons/trash.svg", include_bytes!("../assets/icons/trash.svg")),
    ("icons/triangle-alert.svg", include_bytes!("../assets/icons/triangle-alert.svg")),
    ("icons/wave-sine.svg", include_bytes!("../assets/icons/wave-sine.svg")),
];

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some((_, bytes)) = OWN.iter().find(|(p, _)| *p == path) {
            return Ok(Some(Cow::Borrowed(bytes)));
        }
        gpui_component_assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut v = gpui_component_assets::Assets.list(path)?;
        v.extend(OWN.iter().filter(|(p, _)| p.starts_with(path)).map(|(p, _)| SharedString::from(*p)));
        Ok(v)
    }
}
