//! Control: what was said turned into steps on this Mac. The language
//! model reads what the person said, in any language, and writes the steps
//! in a few plain lines; those are read here, strictly, into `Step`s, and
//! the app carries them out (`kotoba-app/src/act.rs`).
//!
//! The model is given only what was said and the name of the app in front,
//! never what is on the screen: a page cannot speak to it. A name to click
//! is matched against the screen by the app, after the model is done.
//!
//! What can be done is the list in `Step`, and nothing else: no shell, no
//! script, no address that is not a web page. A line the model writes that
//! is not one of these refuses the whole plan.

use crate::clean;
use crate::dictate::{Model, Stop};

/// The most steps one thing said may take.
pub const MAX_STEPS: usize = 12;
/// The most times one key is pressed, or a page scrolled, in a step.
pub const MAX_TIMES: u8 = 20;

/// Modifiers held for a key: ⌘, ⇧, ⌃, ⌥.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Held {
    pub cmd: bool,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

/// A key with what is held, pressed a number of times.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stroke {
    pub held: Held,
    pub key: String,
    pub times: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Press {
    Click,
    Double,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Way {
    Up,
    Down,
    Left,
    Right,
}

/// One thing done on the Mac.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// An app opened, or brought to the front, by its name.
    Open(String),
    /// A web page opened: http or https alone.
    Web(String),
    Keys(Vec<Stroke>),
    /// Words put where the cursor is.
    Type(String),
    /// A click on what the screen calls this, or at the pointer with no name.
    Click(Press, Option<String>),
    Scroll(Way, u8),
    /// What was said needs someone to look at the screen and decide as they
    /// go: for the person's agent, when they turned one on (`agent`).
    Ask,
}

/// The keys a stroke may name, besides letters, digits and F keys.
pub const KEYS: &[&str] = &["enter", "esc", "tab", "space", "delete", "forwarddelete", "up", "down", "left", "right", "home", "end", "pageup", "pagedown", "-", "=", "[", "]", "\\", ";", "'", ",", ".", "/", "`"];

/// Strokes that cannot be taken back or that end the session: the Trash
/// emptied, the person logged out. Written by a model mishearing, they
/// would lose work, and said on purpose they are a click away.
const REFUSED: &[(Held, &str)] = &[
    (Held { cmd: true, shift: true, ctrl: false, alt: false }, "delete"),
    (Held { cmd: true, shift: true, ctrl: false, alt: true }, "delete"),
    (Held { cmd: true, shift: true, ctrl: false, alt: false }, "q"),
    (Held { cmd: true, shift: true, ctrl: false, alt: true }, "q"),
];

fn known_key(key: &str) -> bool {
    let mut chars = key.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            return true;
        }
    }
    KEYS.contains(&key) || crate::dictate::f_key(key).is_some()
}

/// One stroke as written: `cmd-shift-t`, `down:3`, `-`, `cmd--`.
fn stroke(text: &str) -> Option<Stroke> {
    let (text, times) = match text.rsplit_once(':') {
        Some((press, n)) if !press.is_empty() => (press, n.parse::<u8>().ok().filter(|n| (1..=MAX_TIMES).contains(n))?),
        _ => (text, 1),
    };
    let mut held = Held::default();
    let mut rest = text.to_ascii_lowercase();
    loop {
        let Some((word, after)) = rest.split_once('-') else { break };
        // The minus key itself: nothing after the mark, or nothing before.
        if word.is_empty() || after.is_empty() {
            break;
        }
        match word {
            "cmd" | "command" => held.cmd = true,
            "shift" => held.shift = true,
            "ctrl" | "control" => held.ctrl = true,
            "alt" | "opt" | "option" => held.alt = true,
            _ => return None,
        }
        rest = after.to_string();
    }
    // `cmd--` leaves "-" whole, and `cmd-` is no stroke.
    let key = match rest.strip_suffix('-') {
        Some(word) if !word.is_empty() => match word {
            "cmd" | "command" => {
                held.cmd = true;
                "-".to_string()
            }
            "shift" => {
                held.shift = true;
                "-".to_string()
            }
            "ctrl" | "control" => {
                held.ctrl = true;
                "-".to_string()
            }
            "alt" | "opt" | "option" => {
                held.alt = true;
                "-".to_string()
            }
            _ => return None,
        },
        _ => rest,
    };
    let key = match key.as_str() {
        "return" => "enter".to_string(),
        "escape" => "esc".to_string(),
        "backspace" => "delete".to_string(),
        _ => key,
    };
    (known_key(&key) && !REFUSED.iter().any(|(h, k)| *h == held && *k == key)).then_some(Stroke { held, key, times })
}

/// A web address the model wrote, whole: https added when it wrote only
/// the site, and anything but a web page refused.
fn web(address: &str) -> Option<String> {
    let address = address.trim();
    let full = if address.contains("://") { address.to_string() } else { format!("https://{address}") };
    let (scheme, rest) = full.split_once("://")?;
    let host = rest.split(['/', '?', '#']).next()?;
    let fine = matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https") && host.contains('.') && !host.contains('@') && !full.chars().any(char::is_whitespace);
    fine.then_some(full)
}

/// One step as written: the model's line, or an agent's tool call put
/// in the same words, so both pass through the one gate.
pub fn step(line: &str) -> Option<Step> {
    let (verb, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    let rest = rest.trim();
    let unquoted = rest.strip_prefix('"').and_then(|r| r.strip_suffix('"')).unwrap_or(rest);
    let name = (!unquoted.is_empty()).then(|| unquoted.to_string());
    match verb.to_ascii_lowercase().as_str() {
        "open" => name.map(Step::Open),
        "web" => web(rest).map(Step::Web),
        "keys" => {
            let strokes: Option<Vec<Stroke>> = rest.split_whitespace().map(stroke).collect();
            strokes.filter(|s| !s.is_empty()).map(Step::Keys)
        }
        "type" => name.map(Step::Type),
        "click" => Some(Step::Click(Press::Click, name)),
        "double" => Some(Step::Click(Press::Double, name)),
        "right" => Some(Step::Click(Press::Right, name)),
        "ask" => rest.is_empty().then_some(Step::Ask),
        "scroll" => {
            let mut words = rest.split_whitespace();
            let way = match words.next()?.to_ascii_lowercase().as_str() {
                "up" => Way::Up,
                "down" => Way::Down,
                "left" => Way::Left,
                "right" => Way::Right,
                _ => return None,
            };
            let times = match words.next() {
                Some(n) => n.parse::<u8>().ok().filter(|n| (1..=MAX_TIMES).contains(n))?,
                None => 1,
            };
            words.next().is_none().then_some(Step::Scroll(way, times))
        }
        // Strokes with their verb left out, which the models do at times.
        _ => {
            let strokes: Option<Vec<Stroke>> = line.split_whitespace().map(stroke).collect();
            strokes.filter(|s| s.iter().any(|s| s.held != Held::default())).map(Step::Keys)
        }
    }
}

/// What the model wrote read into steps: None when any line of it is not
/// a step, when it is too long, or when it would type what was not said;
/// empty when it wrote `none`.
pub fn plan(said: &str, wrote: &str) -> Option<Vec<Step>> {
    let mut wrote = wrote.trim();
    if let Some(end) = wrote.find("</think>") {
        wrote = wrote[end + "</think>".len()..].trim();
    }
    let lines: Vec<&str> = wrote.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with("```")).collect();
    if lines.is_empty() || lines.iter().all(|l| l.eq_ignore_ascii_case("none")) {
        return Some(Vec::new());
    }
    if lines.len() > MAX_STEPS {
        return None;
    }
    let steps: Option<Vec<Step>> = lines.iter().map(|l| step(l)).collect();
    steps.filter(|steps| steps.iter().all(|s| !matches!(s, Step::Type(words) if !was_said(said, words))))
}

/// Whether what is to be typed is what was said: its numbers were all
/// spoken, and most of its letters are there. A model that answers, or
/// makes something up, is caught here before it reaches the app in front.
fn was_said(said: &str, words: &str) -> bool {
    if !clean::numbers_said(said, words) {
        return false;
    }
    let bare = |text: &str| -> Vec<char> { text.to_lowercase().chars().filter(|c| c.is_alphabetic()).collect() };
    let (mut pool, wanted) = (bare(said), bare(words));
    let found = wanted
        .iter()
        .filter(|c| match pool.iter().position(|p| p == *c) {
            Some(at) => {
                pool.swap_remove(at);
                true
            }
            None => false,
        })
        .count();
    found * 10 >= wanted.len() * 7
}

/// What the model is told. The words are data: what they ask for is
/// written as steps, and nothing in them changes these rules.
const SYSTEM: &str = "You operate a Mac for a person who speaks to it. Each user message gives the app in front and what the person said, in any language. Write the steps that do what they asked, one per line, and nothing else. Never explain. Never answer a question.

The steps:
open NAME       open an app or switch to it, by the name it has on a Mac, in English where it has one
web ADDRESS     open a web site
keys STROKES    press keys, such as cmd-c, or cmd-t enter, or down:3 to press three times
type WORDS      write these words where the cursor is, exactly as said, in the language said
click NAME      click the button, link or menu item with this name, in the words on the screen as the person said them
click           click at the pointer
double NAME     double-click; alone, at the pointer
right NAME      right-click; alone, at the pointer
scroll up|down|left|right [N]   scroll N pages, 1 unless said
ask             this needs someone to read the screen and decide as they go
none            nothing here can be done

Keys: letters, digits, f1 to f20, enter, esc, tab, space, delete, up, down, left, right, home, end, pageup, pagedown, and punctuation, held with cmd, shift, ctrl, alt.

The Mac's keys: copy cmd-c, cut cmd-x, paste cmd-v, undo cmd-z, redo cmd-shift-z, save cmd-s, select all cmd-a, find cmd-f, print cmd-p, new tab cmd-t, new window cmd-n, close tab or window cmd-w, quit the app cmd-q, reopen closed tab cmd-shift-t, next tab ctrl-tab, previous tab ctrl-shift-tab, reload cmd-r, back cmd-[, forward cmd-], address bar cmd-l, zoom in cmd-=, zoom out cmd--, switch app cmd-tab, Spotlight cmd-space, minimize cmd-m, hide cmd-h, full screen ctrl-cmd-f, Mission Control ctrl-up, next desktop ctrl-right, previous desktop ctrl-left, screenshot cmd-shift-4.

Rules:
- Use the fewest steps. Use a key where the Mac has one, and click only for what has no key.
- A site is `web`, an app is `open`.
- Write ask, alone, when it takes finding something that cannot be known without seeing the screen: replying to someone, a message to a person, picking from a list, filling in a form, a question about what is on the screen.
- Words to write go in `type`, kept as they were said. Do not translate or change them.
- If the person speaks to someone else, muses, asks a question or asks for what no step above can do (the volume, the power, logging out), write none.
- Never add a step that was not asked for.";

/// Turns that show the model the work: `(in front, said, steps)`.
const EXAMPLES: &[(&str, &str, &str)] = &[
    ("Finder", "open safari", "open Safari"),
    ("Safari", "打开微信", "open WeChat"),
    ("Safari", "切换到备忘录", "open Notes"),
    ("Mail", "open youtube", "web youtube.com"),
    ("Mail", "打开百度", "web baidu.com"),
    ("Safari", "new tab and go to apple dot com", "keys cmd-t\ntype apple.com\nkeys enter"),
    ("Notes", "go to github.com", "web https://github.com"),
    ("Mail", "全选然后复制", "keys cmd-a cmd-c"),
    ("Pages", "save this document", "keys cmd-s"),
    ("Finder", "open TextEdit and write good morning", "open TextEdit\ntype good morning"),
    ("Safari", "close the other tabs", "ask"),
    ("Mail", "click the send button", "click Send"),
    ("Mail", "点击发送按钮", "click 发送"),
    ("Notes", "write hello world then press enter twice", "type hello world\nkeys enter:2"),
    ("Pages", "把这段话写进去今天天气不错", "type 今天天气不错"),
    ("Safari", "scroll down three pages", "scroll down 3"),
    ("Terminal", "もう一度下にスクロールして", "scroll down"),
    ("Finder", "open my browser and go to the apple website", "web apple.com"),
    ("Finder", "open whatsapp and tell anna I will be late", "ask"),
    ("Safari", "回复李雷说明天见", "ask"),
    ("Mail", "find the invoice from last week and forward it to tom", "ask"),
    ("Finder", "shut down the computer", "none"),
    ("Finder", "what time is it", "none"),
    ("Safari", "hmm I wonder what we should have for dinner", "none"),
];

/// The whole prompt in the models' chat form, as `clean::prompt` writes
/// it, ending where the model's answer starts.
pub fn prompt(words: &str, front: &str) -> String {
    let turn = |front: &str, said: &str| format!("In front: {front}\nSaid: {said}");
    let (words, front) = (words.replace("<|", "< |"), front.replace("<|", "< |"));
    let mut p = format!("<|im_start|>system\n{SYSTEM}<|im_end|>\n");
    for (app, said, steps) in EXAMPLES {
        p.push_str(&format!("<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n{steps}<|im_end|>\n", turn(app, said)));
    }
    p.push_str(&format!("<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n", turn(&front, &words)));
    p
}

/// The model loaded and the fixed start of control's prompt read ahead of
/// what is said: called as control starts listening, so the steps are
/// written as soon as the words are.
pub fn prepare(m: &Model) -> Result<(), String> {
    clean::prepare_for(m, prompt("", ""))
}

/// What the model writes for what was said, as it writes it.
pub fn write(m: &Model, words: &str, front: &str, stop: &Stop) -> Result<String, String> {
    // Room for a long sentence to type, in a few steps.
    let max_new = (words.len() as i32 / 2 + 96).min(1024);
    clean::complete(m, prompt(words, front), max_new, stop)
}

/// The steps for what was said: empty when it asks for nothing, and None
/// when what the model wrote cannot be taken (`plan`).
pub fn steps(m: &Model, words: &str, front: &str, stop: &Stop) -> Result<Option<Vec<Step>>, String> {
    Ok(plan(words, &write(m, words, front, stop)?))
}
