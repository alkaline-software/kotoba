# The menu bar item and the settings window

About `crates/kotoba-app/src/main.rs`, `settings.rs`, `controls.rs`, `scroll.rs`, the window and permission parts of `kotoba.rs`, and the menu bar item in `sys.rs`.

## Rules

- Kotoba is a menu bar app: `LSUIElement` in every bundle, and `sys::stay_in_menu_bar` at launch, because gpui makes every app a regular one as it starts, over the bundle's own say. While the settings window is open it is a regular app (`sys::in_dock`, from `attach` to `detach`): in the Dock and ⌘-Tab, with its own menu (`app_menus`). A menu bar app's window, once another app was in front, stood under that app's windows with no way back to it but the menu bar.
- Accessibility is given when the event tap can be made, never when `AXIsProcessTrusted` says so: a running process reads that once, and after a grant in System Settings it went on saying no until Kotoba was opened again. A new tap is tried only after the person pressed Allow, or while `AXIsProcessTrusted` says yes: a tap made before Kotoba is in the list has the system ask, at launch, unasked.
- What the window held goes when it is closed (`detach`, on `on_window_closed`), never on `on_window_should_close`: ⌘W closes it from code (`remove_window`), which never asks, and left Kotoba in the Dock.
- There is one `Kotoba` for the process's life, held as a global (`main.rs`). The settings window draws it while it is open; closing the window drops only what belongs to a window (its fields, `detach`). Anything dictation does runs on `cx.spawn`, never `spawn_in` a window.
- A field belongs to a window, so the Preferences, Terms and Avoid fields are made each time the window opens (`attach`). Nothing else holds them.
- Opening the window from inside a `Kotoba` update is deferred (`show_settings`): the window's build updates `Kotoba` too.
- Reach a window's content through `cx.update_window`, not `WindowHandle::update`, when keys or actions are dispatched in it: the handle's update leases the root view, and a key dispatched through it panicked ("cannot update Root while it is already being updated").
- Every click the window swallows goes through `swallow_click`.
- A sheet on a scrim that closes at a click occludes (`.occlude()` on the sheet, not only on the scrim).
- A hover style that changes the text's color is set whatever state the element is in, never under a `when`.
- Permissions are asked when the person presses Allow, never on a timer.
- One Kotoba runs at a time: the one opened last ends the others at launch (`sys::end_other_kotobas`), by asking them to quit, so their models are let go, and making them a few seconds on.
- A menu item with an image sets `preferredImageVisibility` to visible, or on macOS 27 the image is not drawn.

## The menu bar item

`sys::menu_bar` puts the item at the menu bar's right: the icon's leaf as a template image (`assets/icons/menubar.png`, drawn by `make-icon.sh`), which the system tints. A left click opens the settings where they were left, and a right click (or Control and a click) shows its menu: the four sections, each with the rail's icon, a line before About, then Quit Kotoba. General, App can swap the two clicks (Swap clicks, `app.bar_swap`) and hide the item (Show in menu bar, `app.bar_icon`); both change at once (`sys::set_menu_bar`). Opened again from Finder or Spotlight while running, Kotoba shows the settings (`on_reopen`), which is the way back to them with the item hidden.

The item has no menu of its own: the button sends both clicks to an Objective-C target (`KotobaMenuTarget`, `define_class!`), which reads the click from the app's current event and either sends the settings down a channel to the app or sets the menu, clicks the button and takes the menu off again once it closes (`show_menu`). An item with a menu set would show it for either click. The menu's choices call the same target, each known by its tag.

The menu's icons are the window's Tabler SVGs, drawn by resvg (the copy gpui builds) at one and two pixels a point into a template image, so the system tints them for the light, the dark and the highlighted line. An app built on the macOS 27 SDK has its menus' images hidden unless each item asks for its own (`preferredImageVisibility`, set by message where the system has it); without that the menu shows names alone.

## The settings window

The window is 820 by 664 (620 until General took Input, which it is grown by), its title bar clear with the traffic lights over the rail, the rail of sections at the left (`SECTIONS`) and the section at the right, fading in as the section changes. The section's name stands in the window's top, level with the traffic lights, as the Mac's own settings have it; the top is the app's to drag by (`drag_region`, with `app_owns_titlebar_drag`), and the rail drags it too. Every section fits the window without scrolling, and the page still scrolls, under the top, for what a fold opens or what is added later.

**Groups and rows.** A section is groups of rows on cards (`group`, `card`, `line`), drawn to `docs/design.md`, which has the sizes.

- **General** is three groups. Dictation: Shortcut (the key, More and Reset; what is said of the key and the guide for Fn or F1 to F12 fold open under the row, across the card), Input (the system's microphone, by the name of the one it has, or one chosen; read again whenever the window comes back and as its list opens, off the main thread, since a Bluetooth one can be slow to answer) and History (how long, and Show History). App: Theme, Launch at login, Show in menu bar and Swap clicks. Permissions: Microphone and Accessibility, asked here ahead of the first dictation, so it does not stop to ask then. Each says whether it is given, and the one thing to do when it is not: Allow (the system's own question), or Open Settings once the microphone was refused, which only System Settings undoes. The rail's General wears a dot while either is missing.
- **Launch at login** is the system's to keep (`SMAppService`, the bundle this process runs from), so it is read from the system when the window opens or comes back (`sys::login`) and never written to the settings: the person may turn it off in Login Items. While the system waits on the person there, Login Items stands before the switch. A bare executable cannot be a login item; try it in the bundle.
- **Models** is the Speech to Text and Post-processing groups, every model on its group's card in one order that stays put as one is chosen, the most capable first. The one in use is washed with the accent; a click on a model that is here takes it up. Post-processing's switch stands at its group's name, and with it off the group fades to gray. Each lane folding to its chosen model was tried: what was folded away was hidden.
- **Customize** is Preferences, a field as tall as the window leaves room for (`PREFERENCES_ROOM`, from the height drawn last), which scrolls inside itself once what is typed is longer than it, and Words: Terms and Avoid (the words never written), a row each with how many there are and Edit. Each opens its list on a sheet (`render_words`): a field with Add, several words at once, and the words under it as chips with a × each, coming and going in motion. At the sheet's foot, Copy All puts the list on the clipboard as one line, a comma and a space between words, and Delete All, in the danger's color, asks in its place ("Delete all 5 words?", Cancel, Delete) before the words fade out. A field that grew with what was typed, and the lists shown on the page, were both tried: the page changed length as it was used.
- **About** is the title card, the credits, and under them the updates: one card of two rows, Updates (what a check found, and one button that is Check for updates until a check finds something and Update to x after, with Release notes under the row's name then) and Check automatically. The rail's About wears a dot while a newer Kotoba waits. Tried and dropped: Updates as a section of its own, which read as part of About.

**More** drops a list of the kinds of key dictation can be pressed alone (`KeyGroup`): One Modifier, Two Modifiers and F Keys, the key in use shown at the right of its kind. Pointed at or pressed, a kind opens its keys in a list beside, as a Mac's submenu does (`render_key_group`): at the side with room, its first key level with the kind's row, and the kind open before going out as the next comes in. The modifiers are on their caps; the F keys stand four to a row, as a keyboard groups them, with a line under F12, where a Mac's own keys end. The kinds' rows are measured as they are drawn (`key_group_at`) for where the list beside stands.

A notice (something refused, a model to download first) opens down from the section's title and closes after a while (`say`, `say_error`).

History, Terms and Avoid open on sheets over the window (`over_settings`): smaller than it both ways, rising in as they fade in, fading out where they were.

## Permissions

The microphone: `sys::mic` reads `AVCaptureDevice`'s status, and `sys::ask_mic` asks. It is read again whenever the window comes back, so a change made in System Settings shows at once.

Accessibility: the first press of Allow in a launch has the system ask (`sys::ask_trust`, `AXIsProcessTrustedWithOptions` with its prompt), which puts Kotoba in the list and offers to open it; the system asks only once in an app's life, so a later press opens Privacy & Security, Accessibility itself. Opening the list on the first press as well stood it under the system's question.

The list is watched for the app's life (`watch_trust`). The system posts `com.apple.accessibility.api` when it changes, before the change is written, and can refuse a new event tap for a moment after a grant, so each notice is followed by reads over a few seconds (`refresh_trust`). Reading for only a second after one notice missed a grant, which then showed only after a second toggle. Each read is the tap itself: the old one switched back on, else a new one made, and Accessibility is given exactly when one of them takes; with neither, the row is back to Allow. `AXIsProcessTrusted` was asked first once, and kept the running Kotoba at "no" after the grant: the process reads it once. The paste goes by the tap's state too. It is read again whenever the window comes back.

A tap made while Kotoba is not in the list has the system ask, as Allow does. Tried at launch on a Mac where Kotoba was never asked, that was the question with no press. So a new tap is made only once Allow was pressed in this launch (after the system's question, a refused tap asks nothing), or while `AXIsProcessTrusted` says yes; the first read in a launch is the system's as it is then. Switched on in System Settings in a launch where Allow was not pressed, the row shows it at the next press of Allow, or the next launch.

**Coming back.** The system's question for the microphone and System Settings take the screen, and when they are done macOS hands it to the app used before, not to Kotoba. So once the microphone is answered, or Accessibility given after the person pressed Allow, the settings come back in front (`come_back`), where the next permission is.

On a first launch (no `config.json` yet) with either missing, the settings open on General. A dictation started with the microphone refused, or one that heard exact zeros, opens them there too.

## Motion

Every motion eases with `ease_out_quint`, at the lengths in `widgets.rs` (`MENU_IN`, `MENU_OUT`, `FOLD_ANIM`). A list over the window comes in over `MENU_IN`, rising into place, and goes over `MENU_OUT`; a choice that moves slides one plate (`segmented`); a switch's knob slides as its track fills with the accent (`switch`); what opens under a row runs its height up from nothing (`fold_under`); something small that comes and goes is played by `presence`; a sheet rises as it fades in.

Three rules hold in every pattern. The first sight plays nothing: a change is played once, keyed on what changed (a serial for each opening of a list, the new choice), so a control drawn again later does not play its change over. Going is drawn from what it last showed, taking no click meanwhile. The room moves with the thing: the gap around a fold is inside what opens and closes, so what is around it slides and does not jump.

## Scroll routing

`scroll.rs`. `route_scroll` runs in the capture phase from a raw listener a zero-size canvas registers at paint. Of the scrollers that name themselves as they are drawn (`inner_scroller`, `kept_scroll`), the one under the pointer drawn last is on top. A gesture begun in one is its own wherever the pointer is carried; a sheet itself (`Inner::Wall`) keeps the wheel from what is under it, so a short sheet does not scroll the settings. The Preferences field names a wall over itself too, after the page's scroller, so a wheel on it scrolls the field and never the page.

A scroll a click starts glides (`settings_glide_step`): when Fn's guide opens at the page's foot, the page runs to its end a share of what is left each frame, never a jump.

## Languages

The window speaks English, Chinese (Simplified and Traditional, told apart by script or region), Japanese, Korean, French, German or Spanish. By default it follows the system: the first of its preferred languages that Kotoba has (`i18n::pick`), else English. Only the window's words change: what dictation hears and writes is the person's own.

**The language list** stands at the right of General's title, on the window's top: System first, then each language by its flag and letters (`i18n::CHOICES`: 🇺🇸 EN, 🇨🇳 ZH-CN, 🇭🇰 ZH-HK for Traditional Chinese, 🇯🇵 JA, 🇰🇷 KO, 🇫🇷 FR, 🇩🇪 DE, 🇪🇸 ES). A choice takes at once (`Kotoba::set_language`): the words are read from the table at every phrase, the menu bar menu is named again (`sys::retitle_menu`), and the window fades into the new language. The choice is the app's own `AppleLanguages` in its defaults, the setting System Settings, Language & Region, Applications writes, so the two are one: what either set is what the list shows at the next launch (`i18n::chosen`), and the microphone's question is asked in it. System removes it.

English is the key. A phrase is written in the code in English inside `t("…")`, or `tf("… {name} …", …)` with blanks, or `count(n, "{n} word", "{n} words")`, and `crates/kotoba-app/locales/words.json` holds its other languages. Words that come from the core (the models' lines, the history's lengths) are put through `t` where they are drawn. A sentence with links in it (About) is woven from its translation (`weave`), so each language puts the links where its order wants them.

`i18n`'s tests read the sources and fail on a phrase asked for and not in the table, one in the table no longer asked for, a language missing from a phrase, or a blank that differs from the English.

The bundle declares the languages (`CFBundleLocalizations`), so System Settings, Language & Region, Applications, can give Kotoba its own; and `macos/locales/<lang>.lproj/InfoPlist.strings` puts the microphone's question in each, copied into the bundle by `dev-app.sh`, `make-app.sh` and `release-mac.sh`. The toolkit is told the language too (`gpui_component::set_locale`); the words of a field's menu were added to its table for the languages it lacked (`vendor/gpui-component/ UPSTREAM.md`, 31).

French and German run longest: a control holds them by growing to its word (`min_w`), never by a width its English fits. Check a change in French, German and Spanish (`KOTOBA_LANG`, `docs/probing.md`).

