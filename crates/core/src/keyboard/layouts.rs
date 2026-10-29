//! Les trois dispositions décrites touche par touche, et les doigts conseillés
//! (méthode classique à dix doigts, identique pour toutes les dispositions).

use super::{Finger, KeyDef, KeyboardLayout, Layout};

/// Description compacte d'une touche : code, base, Maj, AltGr, base morte.
type Spec = (
    &'static str,
    &'static str,
    Option<&'static str>,
    Option<&'static str>,
    bool,
);

/// Doigt conseillé pour un code de touche.
fn finger(code: &str) -> Finger {
    match code {
        "Backquote" | "Digit1" | "Tab" | "KeyQ" | "CapsLock" | "KeyA" | "ShiftLeft"
        | "IntlBackslash" | "KeyZ" | "ControlLeft" => Finger::LeftPinky,
        "Digit2" | "KeyW" | "KeyS" | "KeyX" => Finger::LeftRing,
        "Digit3" | "KeyE" | "KeyD" | "KeyC" => Finger::LeftMiddle,
        "Digit4" | "Digit5" | "KeyR" | "KeyT" | "KeyF" | "KeyG" | "KeyV" | "KeyB" => {
            Finger::LeftIndex
        }
        "Space" | "AltLeft" => Finger::LeftThumb,
        "AltRight" => Finger::RightThumb,
        "Digit6" | "Digit7" | "KeyY" | "KeyU" | "KeyH" | "KeyJ" | "KeyN" | "KeyM" => {
            Finger::RightIndex
        }
        "Digit8" | "KeyI" | "KeyK" | "Comma" => Finger::RightMiddle,
        "Digit9" | "KeyO" | "KeyL" | "Period" => Finger::RightRing,
        _ => Finger::RightPinky,
    }
}

/// Largeur des touches spéciales.
fn width(code: &str) -> f32 {
    match code {
        "Backspace" => 2.0,
        "Tab" => 1.5,
        "CapsLock" => 1.75,
        "Enter" => 1.75,
        "ShiftLeft" => 1.25,
        "ShiftRight" => 2.25,
        "ControlLeft" | "ControlRight" | "AltLeft" | "AltRight" => 1.25,
        "Space" => 6.0,
        _ => 1.0,
    }
}

/// Touche à partir de sa description.
fn key((code, base, shift, altgr, dead): Spec) -> KeyDef {
    KeyDef {
        code: code.to_owned(),
        base: base.to_owned(),
        shift: shift.map(str::to_owned),
        altgr: altgr.map(str::to_owned),
        dead,
        finger: finger(code),
        width: width(code),
    }
}

/// Touche de lettre : la majuscule avec Maj.
const fn letter(code: &'static str, lower: &'static str, upper: &'static str) -> Spec {
    (code, lower, Some(upper), None, false)
}

/// Touche de fonction, désignée par son nom.
const fn named(code: &'static str, name: &'static str) -> Spec {
    (code, name, None, None, false)
}

/// Rangée du bas, commune aux trois dispositions.
fn bottom_row() -> Vec<KeyDef> {
    [
        named("ControlLeft", "Ctrl"),
        named("AltLeft", "Alt"),
        named("Space", "Espace"),
        named("AltRight", "AltGr"),
        named("ControlRight", "Ctrl"),
    ]
    .into_iter()
    .map(key)
    .collect()
}

/// AZERTY français (PC).
fn azerty() -> Vec<Vec<KeyDef>> {
    let rows: [Vec<Spec>; 4] = [
        vec![
            ("Backquote", "²", None, None, false),
            ("Digit1", "&", Some("1"), None, false),
            ("Digit2", "é", Some("2"), Some("~"), false),
            ("Digit3", "\"", Some("3"), Some("#"), false),
            ("Digit4", "'", Some("4"), Some("{"), false),
            ("Digit5", "(", Some("5"), Some("["), false),
            ("Digit6", "-", Some("6"), Some("|"), false),
            ("Digit7", "è", Some("7"), Some("`"), false),
            ("Digit8", "_", Some("8"), Some("\\"), false),
            ("Digit9", "ç", Some("9"), Some("^"), false),
            ("Digit0", "à", Some("0"), Some("@"), false),
            ("Minus", ")", Some("°"), Some("]"), false),
            ("Equal", "=", Some("+"), Some("}"), false),
            named("Backspace", "⌫"),
        ],
        vec![
            named("Tab", "⇥"),
            letter("KeyQ", "a", "A"),
            letter("KeyW", "z", "Z"),
            ("KeyE", "e", Some("E"), Some("€"), false),
            letter("KeyR", "r", "R"),
            letter("KeyT", "t", "T"),
            letter("KeyY", "y", "Y"),
            letter("KeyU", "u", "U"),
            letter("KeyI", "i", "I"),
            letter("KeyO", "o", "O"),
            letter("KeyP", "p", "P"),
            ("BracketLeft", "^", Some("¨"), None, true),
            ("BracketRight", "$", Some("£"), Some("¤"), false),
            named("Enter", "↵"),
        ],
        vec![
            named("CapsLock", "⇪"),
            letter("KeyA", "q", "Q"),
            letter("KeyS", "s", "S"),
            letter("KeyD", "d", "D"),
            letter("KeyF", "f", "F"),
            letter("KeyG", "g", "G"),
            letter("KeyH", "h", "H"),
            letter("KeyJ", "j", "J"),
            letter("KeyK", "k", "K"),
            letter("KeyL", "l", "L"),
            letter("Semicolon", "m", "M"),
            ("Quote", "ù", Some("%"), None, false),
            ("Backslash", "*", Some("µ"), None, false),
        ],
        vec![
            named("ShiftLeft", "⇧"),
            ("IntlBackslash", "<", Some(">"), None, false),
            letter("KeyZ", "w", "W"),
            letter("KeyX", "x", "X"),
            letter("KeyC", "c", "C"),
            letter("KeyV", "v", "V"),
            letter("KeyB", "b", "B"),
            letter("KeyN", "n", "N"),
            ("KeyM", ",", Some("?"), None, false),
            ("Comma", ";", Some("."), None, false),
            ("Period", ":", Some("/"), None, false),
            ("Slash", "!", Some("§"), None, false),
            named("ShiftRight", "⇧"),
        ],
    ];
    rows.into_iter()
        .map(|r| r.into_iter().map(key).collect())
        .chain(std::iter::once(bottom_row()))
        .collect()
}

/// QWERTY américain.
fn qwerty() -> Vec<Vec<KeyDef>> {
    // Rangée de lettres à partir de couples (code, lettre).
    let letters = |codes: &[(&'static str, &'static str, &'static str)]| -> Vec<Spec> {
        codes.iter().map(|&(c, l, u)| letter(c, l, u)).collect()
    };
    let mut row1 = vec![named("Tab", "⇥")];
    row1.extend(letters(&[
        ("KeyQ", "q", "Q"),
        ("KeyW", "w", "W"),
        ("KeyE", "e", "E"),
        ("KeyR", "r", "R"),
        ("KeyT", "t", "T"),
        ("KeyY", "y", "Y"),
        ("KeyU", "u", "U"),
        ("KeyI", "i", "I"),
        ("KeyO", "o", "O"),
        ("KeyP", "p", "P"),
    ]));
    row1.extend([
        ("BracketLeft", "[", Some("{"), None, false),
        ("BracketRight", "]", Some("}"), None, false),
        ("Backslash", "\\", Some("|"), None, false),
    ]);
    let mut row2 = vec![named("CapsLock", "⇪")];
    row2.extend(letters(&[
        ("KeyA", "a", "A"),
        ("KeyS", "s", "S"),
        ("KeyD", "d", "D"),
        ("KeyF", "f", "F"),
        ("KeyG", "g", "G"),
        ("KeyH", "h", "H"),
        ("KeyJ", "j", "J"),
        ("KeyK", "k", "K"),
        ("KeyL", "l", "L"),
    ]));
    row2.extend([
        ("Semicolon", ";", Some(":"), None, false),
        ("Quote", "'", Some("\""), None, false),
        named("Enter", "↵"),
    ]);
    let mut row3 = vec![named("ShiftLeft", "⇧")];
    row3.extend(letters(&[
        ("KeyZ", "z", "Z"),
        ("KeyX", "x", "X"),
        ("KeyC", "c", "C"),
        ("KeyV", "v", "V"),
        ("KeyB", "b", "B"),
        ("KeyN", "n", "N"),
        ("KeyM", "m", "M"),
    ]));
    row3.extend([
        ("Comma", ",", Some("<"), None, false),
        ("Period", ".", Some(">"), None, false),
        ("Slash", "/", Some("?"), None, false),
        named("ShiftRight", "⇧"),
    ]);
    let row0: Vec<Spec> = vec![
        ("Backquote", "`", Some("~"), None, false),
        ("Digit1", "1", Some("!"), None, false),
        ("Digit2", "2", Some("@"), None, false),
        ("Digit3", "3", Some("#"), None, false),
        ("Digit4", "4", Some("$"), None, false),
        ("Digit5", "5", Some("%"), None, false),
        ("Digit6", "6", Some("^"), None, false),
        ("Digit7", "7", Some("&"), None, false),
        ("Digit8", "8", Some("*"), None, false),
        ("Digit9", "9", Some("("), None, false),
        ("Digit0", "0", Some(")"), None, false),
        ("Minus", "-", Some("_"), None, false),
        ("Equal", "=", Some("+"), None, false),
        named("Backspace", "⌫"),
    ];
    [row0, row1, row2, row3]
        .into_iter()
        .map(|r| r.into_iter().map(key).collect())
        .chain(std::iter::once(bottom_row()))
        .collect()
}

/// BÉPO 1.1, légendes principales.
fn bepo() -> Vec<Vec<KeyDef>> {
    let rows: [Vec<Spec>; 4] = [
        vec![
            ("Backquote", "$", Some("#"), None, false),
            ("Digit1", "\"", Some("1"), None, false),
            ("Digit2", "«", Some("2"), None, false),
            ("Digit3", "»", Some("3"), None, false),
            ("Digit4", "(", Some("4"), Some("["), false),
            ("Digit5", ")", Some("5"), Some("]"), false),
            ("Digit6", "@", Some("6"), None, false),
            ("Digit7", "+", Some("7"), None, false),
            ("Digit8", "-", Some("8"), None, false),
            ("Digit9", "/", Some("9"), None, false),
            ("Digit0", "*", Some("0"), None, false),
            ("Minus", "=", Some("°"), None, false),
            ("Equal", "%", Some("`"), None, false),
            named("Backspace", "⌫"),
        ],
        vec![
            named("Tab", "⇥"),
            letter("KeyQ", "b", "B"),
            letter("KeyW", "é", "É"),
            letter("KeyE", "p", "P"),
            ("KeyR", "o", Some("O"), Some("œ"), false),
            letter("KeyT", "è", "È"),
            ("KeyY", "^", Some("!"), None, true),
            letter("KeyU", "v", "V"),
            letter("KeyI", "d", "D"),
            letter("KeyO", "l", "L"),
            letter("KeyP", "j", "J"),
            letter("BracketLeft", "z", "Z"),
            letter("BracketRight", "w", "W"),
            named("Enter", "↵"),
        ],
        vec![
            named("CapsLock", "⇪"),
            ("KeyA", "a", Some("A"), Some("æ"), false),
            letter("KeyS", "u", "U"),
            letter("KeyD", "i", "I"),
            ("KeyF", "e", Some("E"), Some("€"), false),
            ("KeyG", ",", Some(";"), None, false),
            letter("KeyH", "c", "C"),
            letter("KeyJ", "t", "T"),
            letter("KeyK", "s", "S"),
            letter("KeyL", "r", "R"),
            letter("Semicolon", "n", "N"),
            letter("Quote", "m", "M"),
            letter("Backslash", "ç", "Ç"),
        ],
        vec![
            named("ShiftLeft", "⇧"),
            letter("IntlBackslash", "ê", "Ê"),
            letter("KeyZ", "à", "À"),
            letter("KeyX", "y", "Y"),
            letter("KeyC", "x", "X"),
            ("KeyV", ".", Some(":"), None, false),
            letter("KeyB", "k", "K"),
            ("KeyN", "'", Some("?"), Some("’"), false),
            letter("KeyM", "q", "Q"),
            letter("Comma", "g", "G"),
            letter("Period", "h", "H"),
            letter("Slash", "f", "F"),
            named("ShiftRight", "⇧"),
        ],
    ];
    rows.into_iter()
        .map(|r| r.into_iter().map(key).collect())
        .chain(std::iter::once(bottom_row()))
        .collect()
}

/// Disposition complète.
pub fn layout(id: Layout) -> KeyboardLayout {
    let rows = match id {
        Layout::Azerty => azerty(),
        Layout::Qwerty => qwerty(),
        Layout::Bepo => bepo(),
    };
    KeyboardLayout { id, rows }
}
