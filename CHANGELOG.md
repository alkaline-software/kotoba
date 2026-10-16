# Changelog

Each release has a section here, and `release.yml` puts that section on the
GitHub Release as its notes. Write it for the person installing the app, and
keep it short: one line per change, a dozen lines at most, small fixes
grouped into one. The reasoning belongs in AGENTS.md and docs/.

## v0.1.2

- F1 to F20 can be the shortcut on their own: press one in the shortcut field, or choose it under More. A note says when F1 to F12 on an Apple keyboard do what is printed on them instead.
- More opens a list of kinds of key, One Modifier, Two Modifiers and F Keys, each with its keys beside it.
- Only one Kotoba runs at a time: opening one quits any other, so the shortcut no longer works twice.

## v0.1.1

- Kotoba no longer asks for Accessibility on its first launch. The system asks only when you press Allow in Settings, General.
- Closing the settings with ⌘W takes Kotoba out of the Dock, as the close button does.

## v0.1.0

- Kotoba: dictation in any app on your Mac. Press the shortcut, speak, press it again, and the words are pasted where you type, your clipboard put back after. A pill at the top of the screen shows it listening.
- It lives in the menu bar: a click on its icon opens the settings, and a right click shows a menu of each section and Quit. General can swap the two clicks or hide the icon.
- Speech is written down on this Mac by a model you download once (Qwen3-ASR or Whisper), in any language and two mixed; nothing you say leaves the machine.
- Post-processing, when on, removes hesitations, applies corrections and mends grammar with a small local model, keeping to what you said.
- Customize: Preferences in plain words for post-processing, Terms written exactly as you save them, and words to Avoid, never written.
- The shortcut can be a key or a modifier tapped alone, such as Right ⌘ or Fn. Esc cancels while listening.
- A history of what you dictated, kept for as long as you choose.
- The window, its menu and the microphone's question follow your Mac's language, or the one you pick at the top of General: English, Chinese (Simplified and Traditional), Japanese, Korean, French, German or Spanish, else English.
- Settings, General asks for the microphone and Accessibility before your first dictation, and can open Kotoba at login.
