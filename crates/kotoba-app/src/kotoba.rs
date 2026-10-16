//! The app: one `Kotoba` for the process's life. It runs dictation from
//! the shortcut, in whatever app is in front, and it is what the settings
//! window draws while that window is open (`settings.rs`). The window
//! comes and goes; dictation does not go with it, so nothing here waits on
//! a window: its work runs on the app's own clock.
//!
//! Dictation is Emaki's, lifted from its workbench with the composer taken
//! out: the words always go to the app in front, by a paste, and the pill
//! at the top of the screen is the only sign that it is on
//! (`docs/dictation.md`).

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::*;
use gpui_component::input::{InputEvent, InputState, TextareaState};
use gpui_component::Root;
use kotoba_core::config::Config;
use kotoba_core::{dictate, update};

use crate::i18n::{t, tf};
use crate::ui::{binding_signs, MENU_OUT};

/// The settings window's size, and the least it is dragged to.
pub const SETTINGS_W: Pixels = px(820.);
pub const SETTINGS_H: Pixels = px(664.);
const SETTINGS_MIN: Size<Pixels> = Size { width: px(700.), height: px(460.) };

/// The settings' sections, in the rail's order: key, name, icon.
pub const SECTIONS: &[(&str, &str, &str)] = &[
    ("general", "General", "icons/settings.svg"),
    ("models", "Models", "icons/cpu.svg"),
    ("customize", "Customize", "icons/sparkles.svg"),
    ("updates", "Updates", "icons/redo-2.svg"),
    ("about", "About", "icons/info.svg"),
];

/// Dictation's two sounds, Emaki's own (`scripts/dictate-sounds.py`): a
/// chime of two notes rising as it starts to listen, and falling as it
/// stops. A short system sound was tried and read as the Mac's alert.
#[cfg(target_os = "macos")]
const DICTATE_START: &[u8] = include_bytes!("../assets/sounds/dictate-start.wav");
#[cfg(target_os = "macos")]
const DICTATE_STOP: &[u8] = include_bytes!("../assets/sounds/dictate-stop.wav");

/// How many of the microphone's last moments the pill's bars show.
#[cfg(target_os = "macos")]
const MIC_BARS: usize = crate::pill::BARS;

/// How long a notice stands at the head of the settings.
const NOTICE_FOR: Duration = Duration::from_secs(8);

/// A model unused this long is let go: each holds a gigabyte or more.
const IDLE_MODEL: Duration = Duration::from_secs(10 * 60);

/// The key context of the settings window, where Escape is bound.
pub const KEY_CONTEXT: &str = "Kotoba";

actions!(kotoba, [Dictate, Escape, CloseWindow]);

/// Where dictation is: listening while the person speaks, then the model
/// writing it down (`toggle_dictation`). Off a Mac it is never on.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub enum Dictation {
    Off,
    /// The microphone asked for and not yet on.
    Starting,
    #[cfg(target_os = "macos")]
    Listening(dictate::Recorder),
    Writing,
}

/// The person's two lists of words, in Customize: the terms written
/// their way, and the words never written.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum WordList {
    Terms,
    Avoid,
}

impl WordList {
    pub fn key(self) -> &'static str {
        match self {
            WordList::Terms => "terms",
            WordList::Avoid => "avoid",
        }
    }
}

/// The two halves of Models: the models that write down what is said,
/// and the ones that clean it up after.
#[derive(Clone, Copy, PartialEq)]
pub enum Lane {
    Speech,
    Clean,
}

impl Lane {
    pub fn models(self) -> &'static [dictate::Model] {
        match self {
            Lane::Speech => dictate::MODELS,
            Lane::Clean => kotoba_core::clean::MODELS,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Lane::Speech => "speech",
            Lane::Clean => "clean",
        }
    }
}

/// Which list a field opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PickFor {
    /// Dictation's key as a key pressed alone, chosen by its kind
    /// (`KeyGroup`): the More beside its field.
    DictateKey,
    /// How long dictation's history keeps what it wrote.
    DictateKeep,
    /// The microphone dictation listens to.
    Mic,
    /// The window's language, at General's top.
    Language,
}

/// Whether the Mac may do something else with dictation's key, said
/// under its row: Fn, which can have a job of its own, and F1 to F12,
/// which on an Apple keyboard do what is printed on them unless Fn is
/// held. F13 to F20 have no other use.
pub fn key_has_use(key: &str) -> bool {
    dictate::tap(key).is_some_and(|t| t.key == "fn") || dictate::f_key(key).is_some_and(|n| n <= 12)
}

/// The kinds of key More offers for dictation, each a list of its own
/// beside More's: one modifier tapped alone, two tapped together, or an
/// F key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyGroup {
    One,
    Two,
    F,
}

impl KeyGroup {
    pub const ALL: [KeyGroup; 3] = [KeyGroup::One, KeyGroup::Two, KeyGroup::F];

    /// The kind a dictation key is, when More has it.
    pub fn of(key: &str) -> Option<KeyGroup> {
        match dictate::tap(key) {
            Some(tap) if tap.pair() => Some(KeyGroup::Two),
            Some(_) => Some(KeyGroup::One),
            None => dictate::f_key(key).map(|_| KeyGroup::F),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            KeyGroup::One => t("One Modifier"),
            KeyGroup::Two => t("Two Modifiers"),
            KeyGroup::F => t("F Keys"),
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            KeyGroup::One => "one",
            KeyGroup::Two => "two",
            KeyGroup::F => "f",
        }
    }
}

/// A field's list while it is open: every choice, the current one ticked,
/// by the field it hangs off.
#[derive(Clone)]
pub struct PickMenu {
    pub what: PickFor,
    pub rows: Rc<Vec<PickRow>>,
    pub current: String,
    pub by: Bounds<Pixels>,
    pub serial: u32,
}

/// One choice on a list: its key, its name, and a line above it when it
/// is the first of another group.
#[derive(Clone)]
pub struct PickRow {
    pub key: String,
    pub name: String,
    pub rule: bool,
}

/// A scroller on a sheet or a list over the settings. Each says so as it
/// is drawn (`inner_scroller`, `kept_scroll`), and `route_scroll` keeps a
/// gesture with the one it began in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Inner {
    /// On the page, a sheet or a list: a gesture begun off it moves
    /// nothing while the pointer is on it.
    Over,
    /// A sheet itself, drawn before its own scrollers: nothing under it
    /// moves while the pointer is on it, whether or not anything on it
    /// has more to show. A field on it scrolls itself.
    Wall,
}

/// What the settings say of updates: the check, what it found, and the
/// download and install as they go.
#[derive(Default)]
pub struct UpdateView {
    pub checking: bool,
    /// A check has come back since the button was pressed.
    pub answered: bool,
    /// The check under way is the daily one, which speaks only when it
    /// finds something.
    pub automatic: bool,
    pub available: Option<String>,
    pub error: String,
    pub installing: bool,
    pub install_error: String,
    pub progress: Option<(u64, Option<u64>)>,
    pub last_check: f64,
}

/// What the update thread says as it goes.
enum UpdateEvent {
    Checked(Result<String, String>),
    Downloading { done: u64, total: Option<u64> },
    Installing,
    Relaunch,
    InstallFailed(String),
}

/// A line at the head of the settings: something done or refused.
#[derive(Clone)]
pub struct Notice {
    pub words: SharedString,
    pub error: bool,
    pub at: Instant,
}

pub struct Kotoba {
    pub cfg: Config,

    // Dictation as it runs.
    pub dictation: Dictation,
    pub mic_levels: VecDeque<f32>,
    #[cfg(target_os = "macos")]
    pub pill: Option<WindowHandle<crate::pill::Pill>>,
    pub tap_watch: dictate::TapWatch,
    /// Keys pressed in other apps are seen: Accessibility is given and
    /// the tap is on.
    pub outside_on: bool,
    /// Allow was pressed for Accessibility in this launch.
    ax_asked: bool,
    /// The microphone, as the system last said.
    pub mic: crate::sys::Mic,
    /// Whether Kotoba opens at login, as the system last said.
    pub login: crate::sys::Login,
    /// How many times the language changed while running: the window
    /// fades into each new one (`settings.rs`).
    pub lang_serial: u32,

    // The settings window.
    pub window: Option<WindowHandle<Root>>,
    pub window_active: bool,
    pub focus_handle: FocusHandle,
    pub section: &'static str,
    pub settings_scroll: ScrollHandle,
    /// The settings sliding down to Fn's guidance as it opens.
    pub settings_glide: Cell<Option<(Instant, Instant)>>,
    pub notice: Option<Notice>,
    /// What the notice last said, drawn as it closes.
    pub notice_kept: RefCell<SharedString>,
    /// A press on the window's top that may become a move of it.
    pub win_move: bool,

    // Models.
    pub dictate_dl: Vec<(&'static str, Arc<AtomicU64>, Arc<AtomicBool>)>,
    pub dictate_dl_error: Option<String>,

    // The key.
    pub dictate_key_rec: bool,
    pub dictate_key_note: Option<String>,
    pub dictate_key_focus: FocusHandle,
    /// What the notes under the key's row last said, drawn as they close.
    pub dictate_kept: RefCell<(String, &'static str, String)>,
    /// The microphones the Mac has and the name of the one it has for
    /// input, as last read (`read_mics`).
    pub mics: Vec<dictate::Microphone>,
    pub system_mic: Option<String>,
    mic_reads: u32,
    /// What the Mac does with dictation's key besides, once asked (Fn's
    /// job, or `printed` for F1 to F12 doing what is printed on them):
    /// None for nothing.
    pub key_use: Option<Option<&'static str>>,
    key_reads: u32,

    // Lists that drop down.
    pub pick: Option<PickMenu>,
    pub pick_gone: Option<(PickMenu, Instant)>,
    pub pick_serial: u32,
    pub pill_at: Rc<RefCell<HashMap<PickFor, Bounds<Pixels>>>>,
    /// More's list of kinds: the one whose keys are open beside it, by
    /// the change's number (for its motion), the one going, and where
    /// each kind's row was drawn last.
    pub key_group: Option<(KeyGroup, u32)>,
    pub key_group_gone: Option<(KeyGroup, u32, Instant)>,
    pub key_group_at: Rc<RefCell<HashMap<KeyGroup, Bounds<Pixels>>>>,

    // The sheets over the settings.
    pub dictated: Option<(Rc<Vec<dictate::Said>>, u32)>,
    pub dictated_gone: Option<(Rc<Vec<dictate::Said>>, u32, Instant)>,
    pub dictated_serial: u32,
    /// Terms or the words to avoid, open on a sheet, and the one going.
    pub words_sheet: Option<(WordList, u32)>,
    pub words_gone: Option<(WordList, u32, Instant)>,
    /// Delete All pressed on a list's sheet, waiting to be confirmed.
    pub words_confirm: Option<WordList>,
    /// The window's height at the last draw, for what fills it.
    pub viewport_h: Pixels,
    pub clean_prompt: Option<Entity<TextareaState>>,
    pub term_input: Option<Entity<InputState>>,
    pub avoid_input: Option<Entity<InputState>>,
    /// Words just added to a list, which open their room as they come,
    /// and words taken out, drawn closing where they stood.
    pub words_new: Vec<(WordList, String, u32)>,
    pub words_leaving: Vec<(WordList, usize, String, Instant, u32)>,
    pub copied: Option<SharedString>,

    // The controls' own state.
    pub seg_state: RefCell<HashMap<&'static str, (&'static str, &'static str, Option<Instant>)>>,
    pub seg_bounds: Rc<RefCell<HashMap<String, Bounds<Pixels>>>>,
    pub presences: RefCell<HashMap<String, (bool, Instant, u32)>>,
    pub fold_h: RefCell<HashMap<String, Rc<Cell<f32>>>>,

    // Scroll routing.
    pub inner_live: RefCell<Vec<(ScrollHandle, Inner)>>,
    pub inner_kept: RefCell<HashMap<SharedString, (ScrollHandle, bool)>>,
    pub scroll_inner: Option<(ScrollHandle, Inner)>,
    pub scroll_began: bool,
    pub last_scroll: Instant,

    pub update: UpdateView,
}

impl Kotoba {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut cfg = Config::load();
        cfg.app.language = crate::i18n::chosen(&cfg.app.language);
        let me = cx.entity().downgrade();
        // The key's field takes the next keys pressed in it. gpui runs
        // the bindings before an element's own key listener, so they are
        // taken first, here.
        cx.intercept_keystrokes(move |e, _, cx| {
            let Some(me) = me.upgrade() else { return };
            if me.read(cx).dictate_key_rec {
                me.update(cx, |k, cx| k.dictate_key_pressed(&e.keystroke, cx));
                cx.stop_propagation();
            }
        })
        .detach();
        // The settings closed, by the red button or ⌘W alike: what they
        // held goes with them. A close asked for in code (⌘W) never asks
        // `on_window_should_close`, so that alone left Kotoba in the Dock.
        let me = cx.entity().downgrade();
        cx.on_window_closed(move |cx, id| {
            let me = me.clone();
            cx.defer(move |cx| {
                let _ = me.update(cx, |k, cx| {
                    if k.window.is_some_and(|w| w.window_id() == id) {
                        k.detach(cx);
                    }
                });
            });
        })
        .detach();
        // Dictation's key tapped alone (Right ⌘, ⌥⌘): AppKit says which
        // side a modifier is on, and gpui does not. Elsewhere dictation
        // does not run and nothing watches.
        #[cfg(target_os = "macos")]
        {
            let mut keys = crate::sys::key_events();
            cx.spawn(async move |this, cx| {
                use futures::StreamExt as _;
                while let Some(said) = keys.next().await {
                    if this.update(cx, |this, cx| this.key_event(said, cx)).is_err() {
                        break;
                    }
                }
            })
            .detach();
            // And in other apps, once Accessibility allows it.
            cx.spawn(async move |this, cx| {
                let _ = this.update(cx, |this, cx| this.watch_trust(cx));
            })
            .detach();
        }
        // A model unused for a while is let go, and the daily look for a
        // newer Kotoba is made when it is due.
        cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(Duration::from_secs(60)).await;
            dictate::release_idle(IDLE_MODEL);
            kotoba_core::clean::release_idle(IDLE_MODEL);
            if this.update(cx, |this, cx| this.check_updates_daily(cx)).is_err() {
                break;
            }
        })
        .detach();
        let update = UpdateView { last_check: update::UpdateState::load().last_check, ..Default::default() };
        let mut k = Kotoba {
            cfg,
            dictation: Dictation::Off,
            mic_levels: VecDeque::new(),
            #[cfg(target_os = "macos")]
            pill: None,
            tap_watch: dictate::TapWatch::default(),
            outside_on: false,
            ax_asked: false,
            mic: crate::sys::mic(),
            login: crate::sys::login(),
            lang_serial: 0,
            window: None,
            window_active: false,
            focus_handle: cx.focus_handle(),
            section: "general",
            settings_scroll: ScrollHandle::new(),
            settings_glide: Cell::new(None),
            notice: None,
            notice_kept: RefCell::new(SharedString::default()),
            win_move: false,
            dictate_dl: Vec::new(),
            dictate_dl_error: None,
            dictate_key_rec: false,
            dictate_key_note: None,
            dictate_key_focus: cx.focus_handle(),
            dictate_kept: RefCell::new((String::new(), "", String::new())),
            key_use: None,
            key_reads: 0,
            mics: Vec::new(),
            system_mic: None,
            mic_reads: 0,
            pick: None,
            pick_gone: None,
            pick_serial: 0,
            pill_at: Rc::new(RefCell::new(HashMap::new())),
            key_group: None,
            key_group_gone: None,
            key_group_at: Rc::new(RefCell::new(HashMap::new())),
            dictated: None,
            dictated_gone: None,
            dictated_serial: 0,
            words_sheet: None,
            words_gone: None,
            words_confirm: None,
            viewport_h: SETTINGS_H,
            clean_prompt: None,
            term_input: None,
            avoid_input: None,
            words_new: Vec::new(),
            words_leaving: Vec::new(),
            copied: None,
            seg_state: RefCell::new(HashMap::new()),
            seg_bounds: Rc::new(RefCell::new(HashMap::new())),
            presences: RefCell::new(HashMap::new()),
            fold_h: RefCell::new(HashMap::new()),
            inner_live: RefCell::new(Vec::new()),
            inner_kept: RefCell::new(HashMap::new()),
            scroll_inner: None,
            scroll_began: false,
            last_scroll: Instant::now(),
            update,
        };
        k.send_outside_key();
        k.read_key_use(cx);
        k
    }

    // -- the settings window ------------------------------------------------

    /// The settings window brought forward, opened if it is not, on
    /// `section` when one is named.
    pub fn show_settings(&mut self, section: Option<&'static str>, cx: &mut Context<Self>) {
        if let Some(s) = section.filter(|s| SECTIONS.iter().any(|(k, _, _)| k == s)) {
            if self.section != s {
                self.section = s;
                self.settings_scroll.set_offset(point(px(0.), px(0.)));
            }
            self.pick = None;
        }
        self.mic = crate::sys::mic();
        self.login = crate::sys::login();
        self.read_mics(cx);
        cx.notify();
        let this = cx.entity();
        let open = self.window;
        cx.defer(move |cx| {
            cx.activate(true);
            if let Some(h) = open {
                if h.update(cx, |_, window, _| window.activate_window()).is_ok() {
                    return;
                }
            }
            open_window(this, cx);
        });
    }

    /// The window just opened: its fields, which belong to a window, and
    /// what is watched while it is there.
    fn attach(&mut self, handle: WindowHandle<Root>, window: &mut Window, cx: &mut Context<Self>) {
        self.window = Some(handle);
        self.window_active = true;
        crate::sys::in_dock(true);
        let clean_prompt = cx.new(|cx| {
            let mut s = TextareaState::new(window, cx).placeholder(t("Type in your preferences here in plain language."));
            s.set_value(self.cfg.app.clean_prompt.clone(), window, cx);
            s
        });
        cx.subscribe_in(&clean_prompt, window, |this, input, ev: &InputEvent, _, cx| {
            if let InputEvent::Change = ev {
                let mine = input.read(cx).value().to_string();
                if mine != this.cfg.app.clean_prompt {
                    this.cfg.app.clean_prompt = mine.clone();
                    if let Err(e) = Config::edit(move |c| c.app.clean_prompt = mine) {
                        this.say_error(tf("Could not save settings: {e}.", &[("e", &e)]), cx);
                    }
                }
            }
        })
        .detach();
        let words_input = |list: WordList, placeholder: &'static str, window: &mut Window, cx: &mut Context<Self>| {
            let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
            cx.subscribe_in(&input, window, move |this, _, ev: &InputEvent, window, cx| match ev {
                InputEvent::PressEnter { .. } => this.add_words(list, window, cx),
                InputEvent::Change => cx.notify(),
                _ => {}
            })
            .detach();
            input
        };
        self.clean_prompt = Some(clean_prompt);
        self.term_input = Some(words_input(WordList::Terms, t("Accept multiple terms at once"), window, cx));
        self.avoid_input = Some(words_input(WordList::Avoid, t("Accept multiple words at once"), window, cx));
        // A click anywhere else leaves the key's field as it was.
        cx.on_blur(&self.dictate_key_focus, window, |this, _, cx| {
            if std::mem::take(&mut this.dictate_key_rec) {
                this.dictate_key_note = None;
                cx.notify();
            }
        })
        .detach();
        cx.observe_window_activation(window, |this, window, cx| {
            this.window_active = window.is_window_active();
            // A key let go in another app is never seen here.
            if !this.window_active {
                this.tap_watch.reset();
            }
            // Back from Keyboard settings, Fn's use may have changed; back
            // from Privacy & Security, a permission may have.
            if this.window_active {
                this.read_key_use(cx);
                this.mic = crate::sys::mic();
                this.login = crate::sys::login();
                this.read_mics(cx);
                #[cfg(target_os = "macos")]
                this.refresh_trust(cx);
            }
            cx.notify();
        })
        .detach();
        // The palette follows the system only when the settings say so.
        cx.observe_window_appearance(window, |this, window, cx| {
            crate::look::apply(&this.cfg.app.appearance, Some(window), cx);
        })
        .detach();
        crate::look::apply(&self.cfg.app.appearance, Some(window), cx);
        window.focus(&self.focus_handle, cx);
        // A probe's window over the others, where it draws: under another
        // app's it draws nothing and runs no step.
        if std::env::var_os("KOTOBA_GO").is_some() || std::env::var_os("KOTOBA_SHOT").is_some() {
            crate::sys::keep_on_top(window);
        }
    }

    /// The window closed: what it held goes with it.
    fn detach(&mut self, cx: &mut Context<Self>) {
        crate::sys::in_dock(false);
        self.window = None;
        self.window_active = false;
        self.dictate_key_rec = false;
        self.pick = None;
        self.pick_gone = None;
        self.dictated = None;
        self.dictated_gone = None;
        self.words_sheet = None;
        self.words_gone = None;
        self.clean_prompt = None;
        self.term_input = None;
        self.avoid_input = None;
        cx.notify();
    }

    /// A line at the head of the settings, gone after a while.
    pub fn say(&mut self, words: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.notice_set(words.into(), false, cx);
    }

    pub fn say_error(&mut self, words: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.notice_set(words.into(), true, cx);
    }

    fn notice_set(&mut self, words: SharedString, error: bool, cx: &mut Context<Self>) {
        let at = Instant::now();
        self.notice = Some(Notice { words, error, at });
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(NOTICE_FOR).await;
            let _ = this.update(cx, |this, cx| {
                if this.notice.as_ref().is_some_and(|n| n.at == at) {
                    this.notice = None;
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    // -- dictation ----------------------------------------------------------

    /// The dictation key: start listening, or stop and write what was
    /// said where the person is. Nothing is done while the microphone is
    /// starting or the words are being written.
    pub fn toggle_dictation(&mut self, cx: &mut Context<Self>) {
        if !dictate::AVAILABLE {
            self.say(t("Dictation is on a Mac only for now."), cx);
            return;
        }
        match std::mem::replace(&mut self.dictation, Dictation::Off) {
            Dictation::Off => self.start_dictation(cx),
            #[cfg(target_os = "macos")]
            Dictation::Listening(rec) => self.finish_dictation(rec, cx),
            other => self.dictation = other,
        }
    }

    /// Escape while the microphone starts or listens: it stops, and what
    /// it heard is let go unwritten, with no chime: the falling one says
    /// the words are being written. False when there is nothing to stop,
    /// so the key does what it does otherwise.
    pub fn cancel_dictation(&mut self, cx: &mut Context<Self>) -> bool {
        match self.dictation {
            Dictation::Starting => {}
            #[cfg(target_os = "macos")]
            Dictation::Listening(..) => {}
            _ => return false,
        }
        // The recorder dropped closes the microphone; one still starting
        // is dropped when it comes (`start_dictation`).
        self.dictation = Dictation::Off;
        #[cfg(target_os = "macos")]
        {
            crate::sys::take_escape(false);
            self.pill_leave(cx);
        }
        cx.notify();
        true
    }

    /// The microphone on, off the main thread: the first time, the system
    /// asks the person first. With no model here, the settings open where
    /// one is downloaded; with the microphone refused, where it is allowed.
    fn start_dictation(&mut self, cx: &mut Context<Self>) {
        // No model on this Mac (none is shipped with the app): straight to
        // where one is downloaded.
        if dictate::chosen(&self.cfg.app.dictate_model).is_none() {
            self.say(t("Download a speech model first."), cx);
            self.show_settings(Some("models"), cx);
            return;
        }
        #[cfg(target_os = "macos")]
        {
            self.mic = crate::sys::mic();
            if self.mic == crate::sys::Mic::Refused {
                self.say_error(t("Kotoba may not use the microphone. Allow it here."), cx);
                self.show_settings(Some("general"), cx);
                return;
            }
            self.dictation = Dictation::Starting;
            self.mic_levels.clear();
            crate::sys::take_escape(true);
            if self.pill.is_none() {
                self.pill = crate::pill::open(cx);
            }
            // The chime first and the microphone once it has mostly rung,
            // so the chime is not in what it hears.
            crate::sys::play_sound(DICTATE_START);
            cx.notify();
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(Duration::from_millis(300)).await;
                let mic = this.update(cx, |this, _| this.cfg.app.dictate_mic.clone()).unwrap_or_default();
                let rec = cx.background_executor().spawn(async move { dictate::Recorder::start(&mic) }).await;
                let _ = this.update(cx, |this, cx| {
                    match rec {
                        Ok(rec) if matches!(this.dictation, Dictation::Starting) => {
                            this.dictation = Dictation::Listening(rec);
                            this.tick_dictation(cx);
                        }
                        Ok(_) => {}
                        Err(e) => {
                            this.dictation = Dictation::Off;
                            crate::sys::take_escape(false);
                            this.pill_say(t("No microphone"), cx);
                            this.say_error(tf("The microphone did not start: {e}.", &[("e", &e)]), cx);
                        }
                    }
                    cx.notify();
                });
            })
            .detach();
        }
    }

    /// The pill drawn again while it listens, for the loudness.
    #[cfg(target_os = "macos")]
    fn tick_dictation(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(Duration::from_millis(80)).await;
            let on = this
                .update(cx, |this, cx| {
                    let Dictation::Listening(rec) = &this.dictation else { return false };
                    // Eased toward what the microphone last heard, so the
                    // bars swell and fall and do not jump.
                    let heard = (rec.level() / 0.08).clamp(0.0, 1.0);
                    let level = this.mic_levels.back().copied().unwrap_or(0.0) * 0.5 + heard * 0.5;
                    this.mic_levels.push_back(level);
                    while this.mic_levels.len() > MIC_BARS {
                        this.mic_levels.pop_front();
                    }
                    if let Some(pill) = this.pill {
                        let levels = this.mic_levels.clone();
                        let _ = pill.update(cx, |pill, _, cx| pill.set_levels(&levels, cx));
                    }
                    true
                })
                .unwrap_or(false);
            if !on {
                break;
            }
        })
        .detach();
    }

    /// The microphone off, and what it heard written down by the model.
    #[cfg(target_os = "macos")]
    fn finish_dictation(&mut self, rec: dictate::Recorder, cx: &mut Context<Self>) {
        crate::sys::take_escape(false);
        crate::sys::play_sound(DICTATE_STOP);
        self.write_down(move || rec.finish(), cx);
    }

    /// What was heard (`heard`, called off the main thread) written down
    /// by the model, cleaned up when that is on, kept in the history, and
    /// put where the person is.
    #[cfg(target_os = "macos")]
    pub fn write_down(&mut self, heard: impl FnOnce() -> Vec<f32> + std::marker::Send + 'static, cx: &mut Context<Self>) {
        let Some(model) = dictate::chosen(&self.cfg.app.dictate_model) else {
            return;
        };
        let keep = self.cfg.app.dictate_keep.clone();
        // The clean-up's model, when it is on and one is here.
        let cleaner = kotoba_core::clean::chosen(&self.cfg.app.clean_model).filter(|_| self.cfg.app.clean_on);
        let rules = self.cfg.app.clean_prompt.clone();
        let terms = self.cfg.app.dictate_terms.clone();
        let avoided = self.cfg.app.dictate_avoid.clone();
        self.dictation = Dictation::Writing;
        if let Some(pill) = self.pill {
            let _ = pill.update(cx, |pill, _, cx| pill.set_phase(crate::pill::Phase::Writing, cx));
        }
        cx.notify();
        cx.spawn(async move |this, cx| {
            let got = cx
                .background_executor()
                .spawn(async move {
                    let audio = heard();
                    // The system hands silence, not an error, to an app
                    // the person has not let use the microphone.
                    if audio.iter().all(|s| *s == 0.0) {
                        return Err(NotWritten::Silence);
                    }
                    let heard = dictate::spell(&dictate::transcribe(model, &audio, &terms).map_err(NotWritten::Failed)?, &terms);
                    // The words never to be written left out, before the
                    // clean-up sees them and again after it.
                    let kept = dictate::avoid(&heard, &avoided);
                    // Cleaned up when that is on; as it was heard when the
                    // model fails or does not keep to what was said.
                    let mut words = kept.clone();
                    if let Some(cleaner) = cleaner.filter(|_| !kept.is_empty()) {
                        match kotoba_core::clean::tidy(cleaner, &kept, &rules, &terms) {
                            Ok(Some(tidied)) => words = dictate::avoid(&tidied, &avoided),
                            Ok(None) => {}
                            Err(e) => eprintln!("kotoba: post-processing: {e}"),
                        }
                    }
                    if !words.is_empty() {
                        let raw = (words != heard).then(|| heard.clone());
                        let said = dictate::Said { at: chrono::Utc::now().timestamp(), words: words.clone(), secs: audio.len() as f32 / dictate::RATE as f32, model: model.key.to_string(), raw };
                        if let Err(e) = dictate::remember(&said, &keep) {
                            eprintln!("kotoba: the dictation's history: {e}");
                        }
                    }
                    Ok(words)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.dictation = Dictation::Off;
                match got {
                    Ok(words) if words.is_empty() => this.pill_say(t("Nothing heard"), cx),
                    Ok(words) => {
                        this.deliver_dictated(&words, cx);
                        this.refresh_dictated(cx);
                    }
                    Err(NotWritten::Silence) => {
                        this.pill_say(t("No sound"), cx);
                        this.say_error(t("No sound came in. Let Kotoba use the microphone."), cx);
                        this.show_settings(Some("general"), cx);
                    }
                    Err(NotWritten::Failed(e)) => {
                        this.pill_say(t("Nothing written"), cx);
                        this.say_error(format!("{e}."), cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// The words where the person is now. In another app they are pasted,
    /// and what the clipboard held is put back once that app has read it.
    /// In Kotoba's own window the field with the keyboard takes them as
    /// if typed. Where neither can be, they wait on the clipboard.
    #[cfg(target_os = "macos")]
    fn deliver_dictated(&mut self, words: &str, cx: &mut Context<Self>) {
        if self.window_active {
            self.pill_leave(cx);
            let (handle, words) = (self.window, words.to_string());
            // Typed once this update is over: the keys pass through the
            // window's listeners, this one's among them.
            cx.defer(move |cx| {
                let typed = handle.and_then(|h| {
                    // Through the window, not its root view: the keys go
                    // down through that view, which must not be leased.
                    cx.update_window(h.into(), |_, window, cx| {
                        let key = gpui::Keystroke { modifiers: gpui::Modifiers::default(), key: String::new(), key_char: Some(words.clone()) };
                        window.focused(cx).is_some() && window.dispatch_keystroke(key, cx)
                    })
                    .ok()
                });
                // No field to take them: on the clipboard, and in the
                // history, with nothing said over the settings.
                if typed != Some(true) {
                    cx.write_to_clipboard(ClipboardItem::new_string(words));
                }
            });
            return;
        }
        // The tap's state, not `AXIsProcessTrusted`, which a running
        // process reads once (`refresh_trust`).
        if !self.outside_on {
            cx.write_to_clipboard(ClipboardItem::new_string(words.to_string()));
            self.pill_say(t("Copied"), cx);
            self.say_error(t("Allow Accessibility, and the words go where you type."), cx);
            return;
        }
        self.pill_leave(cx);
        let was = crate::sys::paste_words(words);
        cx.spawn(async move |_, cx| {
            cx.background_executor().timer(Duration::from_millis(600)).await;
            cx.update(|_| crate::sys::restore_clipboard(was));
        })
        .detach();
    }

    /// The pill goes back up, its window closing behind it.
    #[cfg(target_os = "macos")]
    pub fn pill_leave(&mut self, cx: &mut Context<Self>) {
        if let Some(pill) = self.pill.take() {
            let _ = pill.update(cx, |pill, window, cx| pill.leave(window, cx));
        }
    }

    /// A few words in the pill, then it goes.
    #[cfg(target_os = "macos")]
    fn pill_say(&mut self, words: &'static str, cx: &mut Context<Self>) {
        let Some(pill) = self.pill.take() else { return };
        let _ = pill.update(cx, |pill, _, cx| pill.set_phase(crate::pill::Phase::Said(words.into()), cx));
        cx.spawn(async move |_, cx| {
            cx.background_executor().timer(Duration::from_millis(1600)).await;
            let _ = pill.update(cx, |pill, window, cx| pill.leave(window, cx));
        })
        .detach();
    }

    /// Whether Accessibility is given, read again, and the tap over other
    /// apps made to match: on when it is given (switched back on, or made
    /// anew), and the settings saying it is not when it was taken away.
    /// Asked at launch, whenever the window comes back, and over a few
    /// seconds after each change the system posts (`watch_trust`).
    #[cfg(target_os = "macos")]
    pub fn refresh_trust(&mut self, cx: &mut Context<Self>) {
        // The tap is the answer. `AXIsProcessTrusted` is read once in a
        // process and kept: switched on in System Settings, it still said
        // no to the running Kotoba, which then never tried the tap, until
        // it was opened again. But a tap made while Kotoba is not yet in
        // Accessibility's list has the system ask, so a new one is tried
        // only once the person pressed Allow, or the system says it is
        // given.
        if crate::sys::revive_outside_keys() {
            if !self.outside_on {
                self.outside_given(cx);
            }
            return;
        }
        let keys = if self.ax_asked || crate::sys::trusted() { crate::sys::outside_keys() } else { None };
        let Some(mut keys) = keys else {
            if self.outside_on {
                self.outside_on = false;
                cx.notify();
            }
            return;
        };
        self.outside_given(cx);
        cx.spawn(async move |this, cx| {
            use futures::StreamExt as _;
            while let Some(said) = keys.next().await {
                if this.update(cx, |this, cx| this.outside_event(said, cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    /// Accessibility's list watched for the app's life. The system posts
    /// `com.apple.accessibility.api` when anything in it changes, before
    /// the change is written, and a tap may be refused a moment longer:
    /// it is read again over a few seconds after each notice, so the keys
    /// work in other apps as soon as Kotoba is switched on, with no need
    /// to come back to it.
    #[cfg(target_os = "macos")]
    fn watch_trust(&mut self, cx: &mut Context<Self>) {
        let mut changes = crate::sys::trust_changes();
        cx.spawn(async move |this, cx| {
            use futures::StreamExt as _;
            while changes.next().await.is_some() {
                for wait in [0, 250, 750, 1000, 2000] {
                    cx.background_executor().timer(Duration::from_millis(wait)).await;
                    if this.update(cx, |this, cx| this.refresh_trust(cx)).is_err() {
                        return;
                    }
                }
            }
        })
        .detach();
        self.refresh_trust(cx);
    }

    /// The tap is on. Given after the person pressed Allow, from System
    /// Settings, Kotoba comes back in front of it, where the next
    /// permission is.
    #[cfg(target_os = "macos")]
    fn outside_given(&mut self, cx: &mut Context<Self>) {
        self.outside_on = true;
        self.send_outside_key();
        if self.ax_asked {
            self.come_back(cx);
        }
        cx.notify();
    }

    /// The settings brought in front again, when they are open: after the
    /// system's question or System Settings took the screen, macOS hands
    /// it to the app used before, not back to Kotoba.
    fn come_back(&mut self, cx: &mut Context<Self>) {
        if self.window.is_some() {
            self.show_settings(None, cx);
        }
    }

    #[cfg(target_os = "macos")]
    fn outside_event(&mut self, said: crate::sys::Outside, cx: &mut Context<Self>) {
        use crate::sys::Outside;
        match said {
            Outside::Flags(flags) => self.key_event(Some(flags), cx),
            Outside::Other => self.key_event(None, cx),
            Outside::Key if !self.dictate_key_rec => self.toggle_dictation(cx),
            Outside::Key => {}
            Outside::Escape => {
                self.cancel_dictation(cx);
            }
        }
    }

    /// Dictation's key, when it is a key, for the tap over other apps to
    /// match.
    fn send_outside_key(&self) {
        #[cfg(target_os = "macos")]
        {
            let key = &self.cfg.app.dictate_key;
            let stroke = if dictate::tap(key).is_some() {
                None
            } else {
                gpui::Keystroke::parse(key).ok().or_else(|| gpui::Keystroke::parse(&kotoba_core::config::AppConfig::default().dictate_key).ok())
            };
            crate::sys::set_outside_key(stroke.as_ref());
        }
    }

    /// A key, a click or the modifiers held changing, as AppKit saw it
    /// (`sys::key_events`): with dictation's key a tap of modifiers
    /// alone, the tap starts or stops it. Not while the key's field is
    /// taking a new key.
    #[cfg(target_os = "macos")]
    fn key_event(&mut self, said: Option<u64>, cx: &mut Context<Self>) {
        if std::env::var_os("KOTOBA_TAP_DEBUG").is_some() {
            eprintln!("kotoba: key event {:?} held {:?} watch {:?}", said.map(|f| format!("{f:#x}")), said.map(dictate::held_from_mac), self.tap_watch);
        }
        let Some(tap) = dictate::tap(&self.cfg.app.dictate_key) else { return };
        if self.dictate_key_rec {
            return self.tap_watch.reset();
        }
        match said {
            Some(flags) => {
                if self.tap_watch.flags(dictate::held_from_mac(flags), tap) {
                    self.toggle_dictation(cx);
                }
            }
            None => self.tap_watch.other(),
        }
    }

    // -- permissions --------------------------------------------------------

    /// The system's question for Accessibility, which puts Kotoba in its
    /// list and offers to open it, at the first press of a launch. The
    /// system asks only once in an app's life, so a later press opens the
    /// list itself; opened on the first press too, the list stood under
    /// the system's question, two windows for one thing.
    pub fn allow_accessibility(&mut self, cx: &mut Context<Self>) {
        #[cfg(target_os = "macos")]
        {
            if std::mem::replace(&mut self.ax_asked, true) {
                cx.open_url(crate::sys::ACCESSIBILITY_SETTINGS_URL);
            } else {
                crate::sys::ask_trust();
            }
            self.refresh_trust(cx);
        }
        cx.notify();
    }

    /// The system's question for the microphone when it was never asked;
    /// after a refusal only System Settings can change it.
    pub fn allow_microphone(&mut self, cx: &mut Context<Self>) {
        #[cfg(target_os = "macos")]
        match crate::sys::mic() {
            crate::sys::Mic::Unasked => {
                let answer = crate::sys::ask_mic();
                cx.spawn(async move |this, cx| {
                    let _ = answer.await;
                    let _ = this.update(cx, |this, cx| {
                        this.mic = crate::sys::mic();
                        this.come_back(cx);
                        cx.notify();
                    });
                })
                .detach();
            }
            crate::sys::Mic::Refused => cx.open_url(crate::sys::MICROPHONE_SETTINGS_URL),
            crate::sys::Mic::Allowed => {}
        }
        cx.notify();
    }

    /// Whether everything dictation needs is given.
    pub fn all_allowed(&self) -> bool {
        self.mic == crate::sys::Mic::Allowed && self.outside_on
    }

    // -- models -------------------------------------------------------------

    /// A model downloaded on its own thread, the settings drawn again as
    /// the bytes come. Any number come at once; one already coming is not
    /// asked for twice.
    pub fn download_model(&mut self, m: &'static dictate::Model, cx: &mut Context<Self>) {
        if self.dictate_dl.iter().any(|(k, _, _)| *k == m.key) {
            return;
        }
        let got = Arc::new(AtomicU64::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let ticking = !self.dictate_dl.is_empty();
        self.dictate_dl.push((m.key, Arc::clone(&got), Arc::clone(&stop)));
        self.dictate_dl_error = None;
        let (tx, rx) = futures::channel::oneshot::channel();
        std::thread::spawn(move || {
            let _ = tx.send(dictate::download(m, &got, &stop));
        });
        // One clock draws them all again while any is coming.
        if !ticking {
            cx.spawn(async move |this, cx| loop {
                cx.background_executor().timer(Duration::from_millis(250)).await;
                let on = this.update(cx, |this, cx| {
                    cx.notify();
                    !this.dictate_dl.is_empty()
                });
                if !on.unwrap_or(false) {
                    break;
                }
            })
            .detach();
        }
        cx.spawn(async move |this, cx| {
            let done = rx.await.unwrap_or_else(|_| Err("the download stopped".into()));
            let _ = this.update(cx, |this, cx| {
                this.dictate_dl.retain(|(k, _, _)| *k != m.key);
                match done {
                    Ok(()) => {}
                    Err(e) if e == "stopped" => {}
                    Err(e) => this.dictate_dl_error = Some(tf("{model} did not finish downloading: {e}.", &[("model", &m.name), ("e", &e)])),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// A lane's model in use: the one chosen when it is here.
    pub fn lane_chosen(&self, lane: Lane) -> Option<&'static dictate::Model> {
        match lane {
            Lane::Speech => dictate::chosen(&self.cfg.app.dictate_model),
            Lane::Clean => kotoba_core::clean::chosen(&self.cfg.app.clean_model),
        }
    }

    /// A model of the lane taken up.
    pub fn lane_use(&mut self, lane: Lane, m: &'static dictate::Model, cx: &mut Context<Self>) {
        match lane {
            Lane::Speech => {
                self.cfg.app.dictate_model = m.key.to_string();
                let _ = Config::edit(move |c| c.app.dictate_model = m.key.to_string());
            }
            Lane::Clean => {
                self.cfg.app.clean_model = m.key.to_string();
                let _ = Config::edit(move |c| c.app.clean_model = m.key.to_string());
            }
        }
        cx.notify();
    }

    /// Post-processing turned on or off. On asks for a model on this Mac:
    /// with none, the list opens where one is downloaded.
    pub fn set_clean_on(&mut self, on: bool, cx: &mut Context<Self>) {
        if on && self.lane_chosen(Lane::Clean).is_none() {
            self.say(t("Download a post-processing model first."), cx);
            return;
        }
        self.cfg.app.clean_on = on;
        let _ = Config::edit(move |c| c.app.clean_on = on);
        cx.notify();
    }

    // -- the key ------------------------------------------------------------

    /// The keys pressed in the dictation key's field: ⌘ or ⌥ with a key,
    /// and ⇧ if wanted, one nothing else in the app has. Esc leaves it as
    /// it was.
    fn dictate_key_pressed(&mut self, keystroke: &Keystroke, cx: &mut Context<Self>) {
        if !self.dictate_key_rec {
            return;
        }
        let m = &keystroke.modifiers;
        let key = keystroke.key.as_str();
        if key == "escape" {
            self.dictate_key_rec = false;
            self.dictate_key_note = None;
            cx.notify();
            return;
        }
        if matches!(key, "shift" | "control" | "alt" | "platform" | "function" | "cmd" | "") {
            return;
        }
        // An F key may be pressed alone; any other key wants ⌘ or ⌥.
        let f = dictate::f_key(key).is_some();
        if (!f && !(m.platform || m.alt)) || m.control || m.function {
            self.dictate_key_note = Some(if cfg!(target_os = "macos") { t("With ⌘ or ⌥.") } else { t("With Ctrl or Alt.") }.into());
            cx.notify();
            return;
        }
        let mut binding = String::new();
        if m.platform {
            binding.push_str("secondary-");
        }
        if m.alt {
            binding.push_str("alt-");
        }
        if m.shift {
            binding.push_str("shift-");
        }
        binding.push_str(key);
        if crate::key_taken(&binding).is_some() {
            self.dictate_key_note = Some(tf("{key} is taken.", &[("key", &binding_signs(&binding))]));
            cx.notify();
            return;
        }
        self.set_dictate_key(binding, cx);
    }

    /// The dictation key back to its first one.
    pub fn reset_dictate_key(&mut self, cx: &mut Context<Self>) {
        self.set_dictate_key(kotoba_core::config::AppConfig::default().dictate_key, cx);
    }

    /// Dictation on `key` from now: a binding, or modifiers tapped alone
    /// (`dictate::TAP_PREFIX`). Saved, and the old key let go at once.
    pub fn set_dictate_key(&mut self, key: String, cx: &mut Context<Self>) {
        self.dictate_key_rec = false;
        self.dictate_key_note = None;
        self.tap_watch.reset();
        if self.cfg.app.dictate_key != key {
            // What the Mac does with the old key says nothing of the new.
            self.key_use = None;
            let old = std::mem::replace(&mut self.cfg.app.dictate_key, key.clone());
            crate::move_dictate_key(&old, &key, cx);
            if let Err(e) = Config::edit(move |c| c.app.dictate_key = key) {
                self.say_error(tf("Could not save settings: {e}.", &[("e", &e)]), cx);
            }
            self.send_outside_key();
        }
        self.read_key_use(cx);
        cx.notify();
    }

    /// With Fn or one of F1 to F12 as dictation's key, ask what the Mac
    /// does with it too, for the guidance under the key's row. Asked
    /// again each time the window comes back, so it goes once the person
    /// has changed it in Keyboard settings.
    pub fn read_key_use(&mut self, cx: &mut Context<Self>) {
        let key = self.cfg.app.dictate_key.clone();
        if !key_has_use(&key) {
            return;
        }
        self.key_reads += 1;
        let n = self.key_reads;
        cx.spawn(async move |this, cx| {
            let job = cx
                .background_spawn(async move {
                    if dictate::f_key(&key).is_some() {
                        (!crate::sys::f_keys_standard()).then_some("printed")
                    } else {
                        crate::sys::fn_key_use()
                    }
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.key_reads != n || this.key_use == Some(job) {
                    return;
                }
                // Coming up under the key's row while the settings show it:
                // the settings slide down to it as it opens.
                if job.is_some() && this.key_use.flatten().is_none() && this.window.is_some() && this.section == "general" {
                    let now = Instant::now();
                    this.settings_glide.set(Some((now, now)));
                }
                this.key_use = Some(job);
                cx.notify();
            });
        })
        .detach();
    }

    // -- the history and what the person tells dictation --------------------

    /// How long dictation's history keeps what it wrote: set, and what
    /// is older let go of now, off the main thread.
    /// The microphones read again, off the main thread: a Bluetooth one
    /// can take a moment to answer. The list of them, open, takes in what
    /// came.
    pub fn read_mics(&mut self, cx: &mut Context<Self>) {
        self.mic_reads += 1;
        let n = self.mic_reads;
        cx.spawn(async move |this, cx| {
            let (mics, system) = cx.background_spawn(async { (dictate::microphones(), dictate::system_microphone()) }).await;
            let _ = this.update(cx, |this, cx| {
                if this.mic_reads != n || (this.mics == mics && this.system_mic == system) {
                    return;
                }
                this.mics = mics;
                this.system_mic = system;
                this.refresh_pick(PickFor::Mic);
                cx.notify();
            });
        })
        .detach();
    }

    /// The microphone dictation listens to: `system` for the one the
    /// system has for input, else one by its id.
    pub fn set_mic(&mut self, id: &str, cx: &mut Context<Self>) {
        let (id, name) = match self.mics.iter().find(|m| m.id == id) {
            Some(m) => (m.id.clone(), m.name.clone()),
            None => (String::new(), String::new()),
        };
        self.cfg.app.dictate_mic = id.clone();
        self.cfg.app.dictate_mic_name = name.clone();
        if let Err(e) = Config::edit(move |c| {
            c.app.dictate_mic = id;
            c.app.dictate_mic_name = name;
        }) {
            self.say_error(tf("Could not save settings: {e}.", &[("e", &e)]), cx);
        }
        cx.notify();
    }

    /// What the microphone's field says: the system's, by the name of the
    /// one it has now, or the one chosen, said when it is not connected.
    pub fn mic_label(&self) -> String {
        let app = &self.cfg.app;
        if app.dictate_mic.is_empty() {
            return match &self.system_mic {
                Some(name) => tf("System ({name})", &[("name", name)]),
                None => t("System").to_string(),
            };
        }
        match self.mics.iter().find(|m| m.id == app.dictate_mic) {
            Some(m) => m.name.clone(),
            None => tf("{name} (not connected)", &[("name", &app.dictate_mic_name)]),
        }
    }

    pub fn set_dictate_keep(&mut self, key: &str, cx: &mut Context<Self>) {
        let key = dictate::keep(key).0.to_string();
        self.cfg.app.dictate_keep = key.clone();
        let _ = Config::edit({
            let key = key.clone();
            move |c| c.app.dictate_keep = key
        });
        cx.spawn(async move |this, cx| {
            let pruned = cx.background_executor().spawn(async move { dictate::prune(&key) }).await;
            let _ = this.update(cx, |this, cx| {
                if let Err(e) = pruned {
                    this.say_error(tf("The history was not trimmed: {e}.", &[("e", &e)]), cx);
                }
                this.refresh_dictated(cx);
            });
        })
        .detach();
        cx.notify();
    }

    /// Dictation's history, opened over the settings.
    pub fn open_dictated(&mut self, cx: &mut Context<Self>) {
        self.dictated_serial += 1;
        self.dictated_gone = None;
        self.dictated = Some((Rc::new(dictate::history(&self.cfg.app.dictate_keep)), self.dictated_serial));
        cx.notify();
    }

    /// The history read again while it is open, keeping its serial so
    /// it does not come in again.
    pub fn refresh_dictated(&mut self, cx: &mut Context<Self>) {
        if let Some((_, serial)) = self.dictated.take() {
            self.dictated = Some((Rc::new(dictate::history(&self.cfg.app.dictate_keep)), serial));
            cx.notify();
        }
    }

    /// The history put away: it fades where it was, taking no click.
    pub fn close_dictated(&mut self, cx: &mut Context<Self>) {
        if let Some((all, serial)) = self.dictated.take() {
            self.dictated_gone = Some((all, serial, Instant::now()));
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(MENU_OUT + Duration::from_millis(20)).await;
                let _ = this.update(cx, |this, cx| {
                    if this.dictated_gone.as_ref().is_some_and(|(_, _, at)| at.elapsed() >= MENU_OUT) {
                        this.dictated_gone = None;
                    }
                    cx.notify();
                });
            })
            .detach();
        }
        cx.notify();
    }

    /// A list opened on a sheet over the settings, its field focused.
    pub fn open_words(&mut self, list: WordList, window: &mut Window, cx: &mut Context<Self>) {
        let Some(input) = self.words_of(list).1 else { return };
        self.dictated_serial += 1;
        self.words_gone = None;
        self.words_sheet = Some((list, self.dictated_serial));
        window.focus(&input.read(cx).focus_handle(cx), cx);
        cx.notify();
    }

    /// The sheet put away: it fades where it was, taking no click, and
    /// the keys go back to the window, as its field is drawn no more.
    pub fn close_words(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((list, serial)) = self.words_sheet.take() else { return };
        self.words_confirm = None;
        self.words_gone = Some((list, serial, Instant::now()));
        window.focus(&self.focus_handle, cx);
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(MENU_OUT + Duration::from_millis(20)).await;
            let _ = this.update(cx, |this, cx| {
                if this.words_gone.is_some_and(|(_, _, at)| at.elapsed() >= MENU_OUT) {
                    this.words_gone = None;
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// A list's words, and its field.
    pub fn words_of(&self, list: WordList) -> (&Vec<String>, Option<Entity<InputState>>) {
        match list {
            WordList::Terms => (&self.cfg.app.dictate_terms, self.term_input.clone()),
            WordList::Avoid => (&self.cfg.app.dictate_avoid, self.avoid_input.clone()),
        }
    }

    fn words_mut(&mut self, list: WordList) -> &mut Vec<String> {
        match list {
            WordList::Terms => &mut self.cfg.app.dictate_terms,
            WordList::Avoid => &mut self.cfg.app.dictate_avoid,
        }
    }

    /// The words in the list's field added to it and saved, and the field
    /// emptied for the next: several at once between commas or spaces
    /// (`dictate::terms_in`). One there already in other capitals takes
    /// the spelling given now.
    pub fn add_words(&mut self, list: WordList, window: &mut Window, cx: &mut Context<Self>) {
        let Some(input) = self.words_of(list).1 else { return };
        let given = dictate::terms_in(input.read(cx).value().as_ref());
        if given.is_empty() {
            return;
        }
        self.dictated_serial += 1;
        let serial = self.dictated_serial;
        self.words_new.retain(|(l, _, _)| *l != list);
        for word in given {
            let words = self.words_mut(list);
            match words.iter().position(|t| t.to_lowercase() == word.to_lowercase()) {
                Some(ix) => words[ix] = word.clone(),
                None => words.push(word.clone()),
            }
            self.words_new.push((list, word, serial));
        }
        self.save_words(list, cx);
        input.update(cx, |s, cx| s.set_value("", window, cx));
        cx.notify();
    }

    /// A word taken out of a list: it fades and closes up where it stood.
    pub fn remove_word(&mut self, list: WordList, ix: usize, cx: &mut Context<Self>) {
        if ix >= self.words_of(list).0.len() {
            return;
        }
        let word = self.words_mut(list).remove(ix);
        self.dictated_serial += 1;
        self.words_leaving.push((list, ix, word, Instant::now(), self.dictated_serial));
        self.save_words(list, cx);
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(crate::ui::FOLD_ANIM + Duration::from_millis(20)).await;
            let _ = this.update(cx, |this, cx| {
                this.words_leaving.retain(|(_, _, _, at, _)| at.elapsed() < crate::ui::FOLD_ANIM);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Every word of a list taken out, once Delete All was confirmed: they
    /// fade and close up where they stood.
    pub fn delete_words(&mut self, list: WordList, cx: &mut Context<Self>) {
        self.words_confirm = None;
        let gone = std::mem::take(self.words_mut(list));
        if gone.is_empty() {
            return cx.notify();
        }
        self.dictated_serial += 1;
        let now = Instant::now();
        for (ix, word) in gone.into_iter().enumerate() {
            self.words_leaving.push((list, ix, word, now, self.dictated_serial));
        }
        self.save_words(list, cx);
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(crate::ui::FOLD_ANIM + Duration::from_millis(20)).await;
            let _ = this.update(cx, |this, cx| {
                this.words_leaving.retain(|(_, _, _, at, _)| at.elapsed() < crate::ui::FOLD_ANIM);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn save_words(&mut self, list: WordList, cx: &mut Context<Self>) {
        let words = self.words_of(list).0.clone();
        let saved = match list {
            WordList::Terms => Config::edit(move |c| c.app.dictate_terms = words),
            WordList::Avoid => Config::edit(move |c| c.app.dictate_avoid = words),
        };
        if let Err(e) = saved {
            self.say_error(tf("Could not save settings: {e}.", &[("e", &e)]), cx);
        }
    }

    /// Put `text` on the clipboard, and have the button `key` show a tick
    /// for a moment.
    pub fn copy_text(&mut self, key: SharedString, text: String, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.copied = Some(key.clone());
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(1500)).await;
            let _ = this.update(cx, |this, cx| {
                if this.copied.as_ref() == Some(&key) {
                    this.copied = None;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    // -- the look -----------------------------------------------------------

    /// System, light or dark: drawn now and kept for the next launch.
    pub fn set_appearance(&mut self, choice: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.cfg.app.appearance = choice.to_string();
        let v = choice.to_string();
        let _ = Config::edit(move |c| c.app.appearance = v);
        crate::look::apply(choice, Some(window), cx);
        cx.notify();
    }

    /// The menu bar icon's two clicks swapped: the left shows its menu.
    pub fn set_bar_swap(&mut self, on: bool, cx: &mut Context<Self>) {
        self.cfg.app.bar_swap = on;
        self.save_bar(cx);
    }

    /// Whether the icon is in the menu bar.
    pub fn set_bar_icon(&mut self, on: bool, cx: &mut Context<Self>) {
        self.cfg.app.bar_icon = on;
        self.save_bar(cx);
    }

    fn save_bar(&mut self, cx: &mut Context<Self>) {
        let (swap, icon) = (self.cfg.app.bar_swap, self.cfg.app.bar_icon);
        crate::sys::set_menu_bar(swap, icon);
        if let Err(e) = Config::edit(move |c| {
            c.app.bar_swap = swap;
            c.app.bar_icon = icon;
        }) {
            self.say_error(tf("Could not save settings: {e}.", &[("e", &e)]), cx);
        }
        cx.notify();
    }

    /// The window's language set to `choice` (`system`, or a code of
    /// `i18n::CHOICES`), at once: the window, its menu bar menu and, on a
    /// Mac, the app's own language in the system's eyes, which its
    /// questions (the microphone's) are asked in from the next launch.
    pub fn set_language(&mut self, choice: &str, cx: &mut Context<Self>) {
        let choice = if choice == "system" || crate::i18n::CHOICES.iter().any(|(c, _, _)| *c == choice) { choice.to_string() } else { "system".to_string() };
        crate::sys::set_own_language((choice != "system").then_some(choice.as_str()));
        crate::i18n::set(&choice);
        crate::sys::retitle_menu(&crate::menu_lines());
        cx.set_menus(crate::app_menus());
        self.cfg.app.language = choice.clone();
        self.lang_serial += 1;
        if let Err(e) = Config::edit(move |c| c.app.language = choice) {
            self.say_error(tf("Could not save settings: {e}.", &[("e", &e)]), cx);
        }
        cx.notify();
    }

    /// Kotoba made to open at login, or not. The system may want the
    /// person's word in Login Items first, which the row then says.
    pub fn set_login(&mut self, on: bool, cx: &mut Context<Self>) {
        if let Err(e) = crate::sys::set_login(on) {
            self.say_error(tf("Could not change opening at login: {e}", &[("e", &e)]), cx);
        }
        self.login = crate::sys::login();
        cx.notify();
    }

    // -- updates ------------------------------------------------------------

    /// Once a day, when the setting allows: the same check the button
    /// makes, but quiet unless it finds something.
    fn check_updates_daily(&mut self, cx: &mut Context<Self>) {
        if !self.cfg.app.check_updates || self.update.checking || self.update.installing {
            return;
        }
        if now_secs() - self.update.last_check < 86_400.0 {
            return;
        }
        self.update.automatic = true;
        self.run_check(cx);
    }

    pub fn check_updates_now(&mut self, cx: &mut Context<Self>) {
        if self.update.checking {
            return;
        }
        self.update.automatic = false;
        self.update.answered = false;
        self.update.error.clear();
        self.run_check(cx);
    }

    /// Ask GitHub for the newest release, on a thread. The time and the
    /// answer are kept in `state/update.json` so the daily check knows
    /// when it last ran.
    fn run_check(&mut self, cx: &mut Context<Self>) {
        self.update.checking = true;
        self.update.last_check = now_secs();
        let (tx, rx) = futures::channel::oneshot::channel();
        std::thread::spawn(move || {
            let got = update::latest();
            if let Ok(latest) = &got {
                let _ = update::UpdateState { last_check: now_secs(), latest: latest.clone() }.save();
            }
            let _ = tx.send(got);
        });
        cx.spawn(async move |this, cx| {
            let got = rx.await.unwrap_or_else(|_| Err("the check stopped".into()));
            let _ = this.update(cx, |this, cx| this.on_update(UpdateEvent::Checked(got), cx));
        })
        .detach();
        cx.notify();
    }

    /// Fetch the newer version's installer and put it in place, on a
    /// thread, reporting as it goes. Ends in a relaunch or a failure.
    pub fn install_update_now(&mut self, cx: &mut Context<Self>) {
        let Some(version) = self.update.available.clone() else { return };
        if self.update.installing {
            return;
        }
        self.update.installing = true;
        self.update.install_error.clear();
        self.update.progress = Some((0, None));
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        std::thread::spawn(move || {
            let progress = |done: u64, total: Option<u64>| {
                let _ = tx.unbounded_send(UpdateEvent::Downloading { done, total });
            };
            let file = match update::download(&version, &progress) {
                Ok(f) => f,
                Err(e) => {
                    let _ = tx.unbounded_send(UpdateEvent::InstallFailed(e));
                    return;
                }
            };
            let _ = tx.unbounded_send(UpdateEvent::Installing);
            let _ = tx.unbounded_send(match update::install(&file) {
                Ok(()) => UpdateEvent::Relaunch,
                Err(e) => UpdateEvent::InstallFailed(e),
            });
        });
        cx.spawn(async move |this, cx| {
            use futures::StreamExt as _;
            while let Some(ev) = rx.next().await {
                if this.update(cx, |this, cx| this.on_update(ev, cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        cx.notify();
    }

    pub fn set_check_updates(&mut self, on: bool, cx: &mut Context<Self>) {
        self.cfg.app.check_updates = on;
        if let Err(e) = Config::edit(move |c| c.app.check_updates = on) {
            self.say_error(tf("Could not save settings: {e}.", &[("e", &e)]), cx);
        }
        cx.notify();
    }

    fn on_update(&mut self, ev: UpdateEvent, cx: &mut Context<Self>) {
        match ev {
            UpdateEvent::Checked(Ok(latest)) => {
                self.update.checking = false;
                self.update.answered = true;
                self.update.available = update::is_newer(&latest, update::current_version()).then_some(latest);
            }
            UpdateEvent::Checked(Err(e)) => {
                self.update.checking = false;
                self.update.answered = true;
                if !self.update.automatic {
                    self.update.error = e;
                }
            }
            UpdateEvent::Downloading { done, total } => self.update.progress = Some((done, total)),
            UpdateEvent::Installing => self.update.progress = None,
            // The new app is in place and starts once this one has left.
            UpdateEvent::Relaunch => cx.quit(),
            UpdateEvent::InstallFailed(e) => {
                self.update.installing = false;
                self.update.progress = None;
                self.update.install_error = e;
            }
        }
        cx.notify();
    }

    // -- probes -------------------------------------------------------------

    /// `KOTOBA_GO`, a step of it: a state of the settings or of dictation
    /// reached from a script, to be looked at (`docs/probing.md`).
    pub fn go_step(&mut self, step: &str, window: &mut Window, cx: &mut Context<Self>) {
        match step {
            "history" => self.open_dictated(cx),
            "close" => {
                self.close_dictated(cx);
                self.close_words(window, cx);
            }
            "terms" => self.open_words(WordList::Terms, window, cx),
            // Copy All on the open sheet.
            "copyall" => {
                if let Some((list, _)) = self.words_sheet {
                    let all = self.words_of(list).0.join(", ");
                    self.copy_text(format!("{}-copy-all", list.key()).into(), all, cx);
                }
            }
            // Delete All on the open sheet, and its confirmation.
            "deleteall" => self.words_confirm = self.words_sheet.map(|(l, _)| l),
            "deleteyes" => {
                if let Some(list) = self.words_confirm {
                    self.delete_words(list, cx);
                }
            }
            "avoid" => self.open_words(WordList::Avoid, window, cx),
            "pick:dictate-key" => self.pick_toggle(PickFor::DictateKey, cx),
            s if s.starts_with("group:") => {
                if let Some(g) = KeyGroup::ALL.into_iter().find(|g| g.key() == &s["group:".len()..]) {
                    self.open_key_group(g, cx);
                }
            }
            "pick:dictate-keep" => self.pick_toggle(PickFor::DictateKeep, cx),
            "pick:mic" => self.pick_toggle(PickFor::Mic, cx),
            "pick:language" => self.pick_toggle(PickFor::Language, cx),
            "dark" => self.set_appearance("dark", window, cx),
            "light" => self.set_appearance("light", window, cx),
            "key" => self.toggle_dictation(cx),
            "reset" => self.reset_dictate_key(cx),
            // The notes that ask for a permission drawn as if neither
            // were given.
            "untrusted" => {
                self.outside_on = false;
                self.mic = crate::sys::Mic::Unasked;
            }
            "get" => {
                if let Some(m) = dictate::MODELS.iter().rev().find(|m| !dictate::installed(m)) {
                    self.download_model(m, cx);
                }
            }
            #[cfg(target_os = "macos")]
            "dictate:pretend" => {
                self.mic_levels.clear();
                self.pill = crate::pill::open(cx);
                self.dictation = Dictation::Listening(dictate::Recorder::pretend());
                self.tick_dictation(cx);
            }
            #[cfg(target_os = "macos")]
            "bar" => eprintln!("kotoba: bar {:?}", crate::sys::menu_bar_frame()),
            #[cfg(target_os = "macos")]
            s if s.starts_with("menushot:") => crate::sys::shoot_menu(std::path::PathBuf::from(&s["menushot:".len()..])),
            s if s.starts_with("language:") => self.set_language(&s["language:".len()..], cx),
            s if s.starts_with("clean:") => self.set_clean_on(&s["clean:".len()..] == "on", cx),
            s if s.starts_with("barswap:") => self.set_bar_swap(&s["barswap:".len()..] == "on", cx),
            s if s.starts_with("login:") => self.set_login(&s["login:".len()..] == "on", cx),
            s if s.starts_with("baricon:") => self.set_bar_icon(&s["baricon:".len()..] == "show", cx),
            #[cfg(target_os = "macos")]
            "away" => eprintln!("kotoba: trusted {} outside {} mic {:?}", crate::sys::trusted(), self.outside_on, crate::sys::mic()),
            s if s.starts_with("section:") => self.show_settings(SECTIONS.iter().find(|(k, _, _)| *k == &s["section:".len()..]).map(|(k, _, _)| *k), cx),
            // Words typed into a list's field and added, as Enter does:
            // `words:avoid:like,so` (a comma for a space).
            s if s.starts_with("words:") => {
                let mut parts = s["words:".len()..].splitn(2, ':');
                let list = if parts.next() == Some("avoid") { WordList::Avoid } else { WordList::Terms };
                if let Some(input) = self.words_of(list).1 {
                    let typed = parts.next().unwrap_or("").replace(',', " ");
                    input.update(cx, |s, cx| s.set_value(typed, window, cx));
                    self.add_words(list, window, cx);
                }
            }
            // The first word of a list taken out: `unword:terms`.
            s if s.starts_with("unword:") => self.remove_word(if s.ends_with("avoid") { WordList::Avoid } else { WordList::Terms }, 0, cx),
            s if s.starts_with("notice:") => self.say(s["notice:".len()..].replace('_', " "), cx),
            s if s.starts_with("fstate:") => {
                crate::sys::F_STATE_PROBE.store(s["fstate:".len()..].parse().unwrap_or(-1), Ordering::Relaxed);
                self.read_key_use(cx);
            }
            s if s.starts_with("fnuse:") => {
                crate::sys::FN_USE_PROBE.store(s["fnuse:".len()..].parse().unwrap_or(-1), Ordering::Relaxed);
                self.read_key_use(cx);
            }
            s if s.starts_with("dictatekey:") => self.set_dictate_key(s["dictatekey:".len()..].to_string(), cx),
            // A 16-bit WAV file written down as the microphone's recording
            // is, and put where the person is.
            #[cfg(target_os = "macos")]
            s if s.starts_with("dictate:file:") => {
                let path = std::path::PathBuf::from(&s["dictate:file:".len()..]);
                self.pill = crate::pill::open(cx);
                self.write_down(
                    move || {
                        let bytes = std::fs::read(&path).unwrap_or_default();
                        dictate::wav_samples(&bytes).map(|(s, ch, rate)| dictate::to_whisper(&s, ch, rate)).unwrap_or_default()
                    },
                    cx,
                );
            }
            #[cfg(target_os = "macos")]
            s if s.starts_with("pillshot:") => {
                let path = std::path::PathBuf::from(&s["pillshot:".len()..]);
                if let Some(pill) = self.pill {
                    let _ = pill.update(cx, |_, window, _| eprintln!("kotoba: pill {:?} {:?}", window.bounds(), crate::sys::shoot_window(window, &path)));
                }
            }
            other => eprintln!("kotoba: KOTOBA_GO: no step {other:?}"),
        }
        cx.notify();
    }
}

/// Why nothing was written: the system handed silence, or the model failed.
#[cfg(target_os = "macos")]
enum NotWritten {
    Silence,
    Failed(String),
}

/// Seconds since 1970, as `state/update.json` keeps them.
fn now_secs() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

/// The settings window: a window of the system's with its title bar
/// clear and its traffic lights over the rail, the size of Emaki's
/// settings panel. Its content is the app's one `Kotoba`.
fn open_window(this: Entity<Kotoba>, cx: &mut App) {
    let bounds = Bounds::centered(None, size(SETTINGS_W, SETTINGS_H), cx);
    let titlebar = if cfg!(target_os = "macos") {
        TitlebarOptions { title: Some("Kotoba".into()), appears_transparent: true, traffic_light_position: Some(point(px(17.), px(19.))) }
    } else {
        TitlebarOptions { title: Some("Kotoba".into()), appears_transparent: false, traffic_light_position: None }
    };
    let opened = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(titlebar),
            window_min_size: Some(SETTINGS_MIN),
            app_id: Some("kotoba".into()),
            // The top of the window is the app's to drag by (`drag_region`):
            // left to AppKit, a press on a control there moved the window.
            app_owns_titlebar_drag: true,
            ..Default::default()
        },
        |window, cx| {
            let handle = window.window_handle().downcast::<Root>();
            this.update(cx, |k, cx| {
                if let Some(h) = handle {
                    k.attach(h, window, cx);
                }
            });
            cx.new(|cx| Root::new(this.clone(), window, cx))
        },
    );
    match opened {
        Ok(handle) => {
            let _ = handle.update(cx, |_, window, _| window.activate_window());
            probe(this, handle, cx);
        }
        Err(e) => eprintln!("kotoba: could not open the settings: {e}"),
    }
}

/// `KOTOBA_GO` steps, five seconds apart and the first a second in, and
/// `KOTOBA_SHOT`, a picture of the window that many seconds in
/// (`KOTOBA_SHOT_AFTER`, five by default), after which the app quits.
/// Each is done for the first window a launch opens.
fn probe(this: Entity<Kotoba>, handle: WindowHandle<Root>, cx: &mut App) {
    static ONCE: AtomicBool = AtomicBool::new(false);
    if ONCE.swap(true, Ordering::Relaxed) {
        return;
    }
    if let Ok(steps) = std::env::var("KOTOBA_GO") {
        let steps: Vec<String> = steps.split(';').map(str::to_string).filter(|s| !s.is_empty()).collect();
        let k = this.clone();
        cx.spawn(async move |cx| {
            for (ix, step) in steps.into_iter().enumerate() {
                cx.background_executor().timer(Duration::from_secs(if ix == 0 { 1 } else { 5 })).await;
                // `wheel:<x>:<y>:<dy>`: the wheel turned there, in points
                // from the window's top left, through the window as the
                // mouse sends it. Sent from here, not from `Kotoba`, which
                // hears it (`route_scroll`).
                if let Some(at) = step.strip_prefix("wheel:") {
                    let n: Vec<f32> = at.split(':').filter_map(|v| v.parse().ok()).collect();
                    if let [x, y, dy] = n[..] {
                        let _ = cx.update_window(handle.into(), move |_, window, cx| {
                            for phase in [TouchPhase::Started, TouchPhase::Moved, TouchPhase::Ended] {
                                let e = ScrollWheelEvent { position: point(px(x), px(y)), delta: ScrollDelta::Pixels(point(px(0.), px(dy / 3.))), modifiers: Modifiers::default(), touch_phase: phase };
                                window.dispatch_event(PlatformInput::ScrollWheel(e), cx);
                            }
                        });
                    }
                    continue;
                }
                // `keys:<keystroke>`: a key pressed in the window, through
                // its bindings (`keys:cmd-w`).
                if let Some(key) = step.strip_prefix("keys:").and_then(|k| Keystroke::parse(k).ok()) {
                    let _ = cx.update_window(handle.into(), move |_, window, cx| window.dispatch_keystroke(key, cx));
                    continue;
                }
                let k = k.clone();
                let _ = cx.update_window(handle.into(), move |_, window, cx| k.update(cx, |k, cx| k.go_step(&step, window, cx)));
            }
        })
        .detach();
    }
    if let Some(path) = std::env::var_os("KOTOBA_SHOT").filter(|p| !p.is_empty()) {
        let after = std::env::var("KOTOBA_SHOT_AFTER").ok().and_then(|v| v.parse::<f64>().ok()).unwrap_or(5.0);
        cx.spawn(async move |cx| {
            cx.background_executor().timer(Duration::from_secs_f64(after)).await;
            let _ = handle.update(cx, |_, window, cx| {
                if let Err(e) = crate::sys::shoot_window(window, std::path::Path::new(&path)) {
                    eprintln!("kotoba: KOTOBA_SHOT: {e}");
                }
                cx.quit();
            });
        })
        .detach();
    }
}
