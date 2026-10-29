//! Dispositions de clavier (F-08, ADR 0022) : touches, légendes, doigt conseillé,
//! et touches à frapper pour obtenir un caractère (touches mortes comprises).

/// Description des trois dispositions.
mod layouts;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

pub use layouts::layout;

/// Disposition ; valeurs identiques à la contrainte `profiles.keyboard_layout`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum Layout {
    /// AZERTY français (PC).
    #[default]
    Azerty,
    /// QWERTY américain.
    Qwerty,
    /// BÉPO (disposition française ergonomique).
    Bepo,
}

impl Layout {
    /// Valeur de la colonne `profiles.keyboard_layout`.
    pub fn as_str(self) -> &'static str {
        match self {
            Layout::Azerty => "azerty",
            Layout::Qwerty => "qwerty",
            Layout::Bepo => "bepo",
        }
    }

    /// Lit la colonne ; une valeur inconnue vaut AZERTY.
    pub fn parse(value: &str) -> Self {
        match value {
            "qwerty" => Layout::Qwerty,
            "bepo" => Layout::Bepo,
            _ => Layout::Azerty,
        }
    }
}

/// Doigt conseillé pour une touche.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum Finger {
    /// Auriculaire gauche.
    LeftPinky,
    /// Annulaire gauche.
    LeftRing,
    /// Majeur gauche.
    LeftMiddle,
    /// Index gauche.
    LeftIndex,
    /// Pouce gauche.
    LeftThumb,
    /// Pouce droit.
    RightThumb,
    /// Index droit.
    RightIndex,
    /// Majeur droit.
    RightMiddle,
    /// Annulaire droit.
    RightRing,
    /// Auriculaire droit.
    RightPinky,
}

/// Une touche du clavier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct KeyDef {
    /// Code physique (`KeyboardEvent.code` : « KeyQ », « Digit1 », « Space »…).
    pub code: String,
    /// Caractère sans modificateur, ou nom de la touche (« Espace »).
    pub base: String,
    /// Caractère avec Maj, s'il diffère.
    pub shift: Option<String>,
    /// Caractère avec AltGr.
    pub altgr: Option<String>,
    /// Le caractère de base est-il une touche morte (^, ¨…) ?
    pub dead: bool,
    /// Doigt conseillé.
    pub finger: Finger,
    /// Largeur, en largeurs de touche ordinaire.
    pub width: f32,
}

/// Disposition complète, rangée par rangée, de haut en bas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct KeyboardLayout {
    /// Disposition décrite.
    pub id: Layout,
    /// Rangées de touches.
    pub rows: Vec<Vec<KeyDef>>,
}

/// Une frappe de touche, avec ses modificateurs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Press {
    /// Code de la touche.
    pub code: String,
    /// Maj enfoncée.
    pub shift: bool,
    /// AltGr enfoncée.
    pub altgr: bool,
    /// Verrouillage des majuscules (AZERTY Windows : É, À, Ç…).
    pub caps: bool,
}

impl Press {
    /// Frappe simple, sans modificateur.
    fn plain(code: &str) -> Self {
        Self {
            code: code.to_owned(),
            shift: false,
            altgr: false,
            caps: false,
        }
    }
}

/// Signe de la touche morte qui pose un accent combinant.
fn dead_sign(mark: char) -> Option<&'static str> {
    Some(match mark {
        '\u{302}' => "^",
        '\u{308}' => "¨",
        '\u{300}' => "`",
        '\u{303}' => "~",
        _ => return None,
    })
}

/// Variante clavier d'un signe typographique (’ → ', « → "…).
fn keyboard_variant(g: &str) -> Option<&'static str> {
    Some(match g {
        "’" | "‘" => "'",
        "«" | "»" | "“" | "”" => "\"",
        "–" | "—" => "-",
        "…" => ".",
        _ => return None,
    })
}

/// Index inverse d'une disposition : caractère → frappe, et touches mortes.
struct Index {
    /// Frappes directes (sans touche morte), par caractère.
    direct: HashMap<String, Press>,
    /// Frappes des touches mortes, par signe.
    dead: HashMap<String, Press>,
}

impl Index {
    /// Construit l'index d'une disposition ; la première touche trouvée l'emporte.
    fn new(layout: &KeyboardLayout) -> Self {
        let mut direct = HashMap::new();
        let mut dead = HashMap::new();
        for key in layout.rows.iter().flatten() {
            let base = Press::plain(&key.code);
            let shifted = Press {
                shift: true,
                ..base.clone()
            };
            let altgr = Press {
                altgr: true,
                ..base.clone()
            };
            // Touche morte : son signe de base (et Maj pour le tréma en AZERTY).
            let table = if key.dead { &mut dead } else { &mut direct };
            table.entry(key.base.clone()).or_insert(base);
            if let Some(s) = &key.shift {
                // Maj sur une touche morte donne une autre touche morte (^ → ¨).
                let t = if key.dead && !s.chars().any(char::is_alphanumeric) {
                    &mut dead
                } else {
                    &mut direct
                };
                t.entry(s.clone()).or_insert(shifted);
            }
            if let Some(a) = &key.altgr {
                direct.entry(a.clone()).or_insert(altgr);
            }
        }
        Self { direct, dead }
    }
}

/// Touches à frapper, dans l'ordre, pour obtenir `grapheme` avec la disposition.
/// Les caractères absents de la disposition sont approchés (« é » en QWERTY → « e ») ;
/// une liste vide signifie qu'aucune touche ne convient.
pub fn keys_for(grapheme: &str, layout: &KeyboardLayout) -> Vec<Press> {
    presses(grapheme, &Index::new(layout))
}

/// Touches de chaque graphème d'une liste (guide du clavier virtuel), en un seul
/// calcul d'index.
pub fn keys_for_all(graphemes: &[String], layout: &KeyboardLayout) -> Vec<Vec<Press>> {
    let index = Index::new(layout);
    graphemes.iter().map(|g| presses(g, &index)).collect()
}

/// Recherche des touches d'un graphème dans l'index.
fn presses(grapheme: &str, index: &Index) -> Vec<Press> {
    // Espaces et retours à la ligne : touches dédiées.
    match grapheme {
        " " | "\u{a0}" | "\u{202f}" => return vec![Press::plain("Space")],
        "\n" | "\r\n" => return vec![Press::plain("Enter")],
        "\t" => return vec![Press::plain("Tab")],
        _ => {}
    }
    // Caractère présent tel quel sur une touche.
    if let Some(p) = index.direct.get(grapheme) {
        return vec![p.clone()];
    }
    // Majuscule absente : minuscule avec Maj, ou verrouillage des majuscules si Maj
    // donne autre chose sur cette touche (é → 2 en AZERTY).
    let lower = grapheme.to_lowercase();
    if lower != grapheme
        && let Some(p) = index.direct.get(&lower)
    {
        let shifted_is_upper = index.direct.get(grapheme).is_some_and(|s| s.code == p.code);
        let letter_key = lower.chars().all(|c| c.is_ascii_alphabetic());
        return vec![Press {
            shift: letter_key || shifted_is_upper,
            caps: !(letter_key || shifted_is_upper),
            ..p.clone()
        }];
    }
    // Lettre accentuée : touche morte, puis la lettre de base.
    let decomposed: Vec<char> = grapheme.nfd().collect();
    let base: String = decomposed
        .iter()
        .filter(|c| !is_combining_mark(**c))
        .collect();
    let marks: Vec<char> = decomposed
        .iter()
        .copied()
        .filter(|c| is_combining_mark(*c))
        .collect();
    if let [mark] = marks[..]
        && !base.is_empty()
        && let Some(dead) = dead_sign(mark).and_then(|s| index.dead.get(s))
    {
        let mut keys = vec![dead.clone()];
        keys.extend(presses(&base, index));
        return keys;
    }
    // Variante typographique (’ → '), puis lettre sans accent (é → e en QWERTY).
    if let Some(v) = keyboard_variant(grapheme) {
        return presses(v, index);
    }
    if !marks.is_empty() && !base.is_empty() {
        return presses(&base, index);
    }
    Vec::new()
}

/// Tests des dispositions et des touches par caractère.
#[cfg(test)]
mod tests {
    use super::*;

    /// Codes des touches et modificateurs d'un caractère : « Shift+KeyQ », « BracketLeft ».
    fn keys(g: &str, l: Layout) -> Vec<String> {
        keys_for(g, &layout(l))
            .into_iter()
            .map(|p| {
                let mut s = String::new();
                if p.caps {
                    s.push_str("Caps+");
                }
                if p.shift {
                    s.push_str("Shift+");
                }
                if p.altgr {
                    s.push_str("AltGr+");
                }
                s + &p.code
            })
            .collect()
    }

    /// AZERTY : lettres, majuscules, chiffres, accents directs et touches mortes.
    #[test]
    fn azerty_keys() {
        let a = Layout::Azerty;
        assert_eq!(keys("q", a), ["KeyA"]);
        assert_eq!(keys("a", a), ["KeyQ"]);
        assert_eq!(keys("Q", a), ["Shift+KeyA"]);
        assert_eq!(keys("m", a), ["Semicolon"]);
        assert_eq!(keys("1", a), ["Shift+Digit1"]);
        assert_eq!(keys("é", a), ["Digit2"]);
        assert_eq!(keys("ù", a), ["Quote"]);
        assert_eq!(keys("É", a), ["Caps+Digit2"]);
        assert_eq!(keys("ê", a), ["BracketLeft", "KeyE"]);
        assert_eq!(keys("Ê", a), ["BracketLeft", "Shift+KeyE"]);
        assert_eq!(keys("ë", a), ["Shift+BracketLeft", "KeyE"]);
        assert_eq!(keys("€", a), ["AltGr+KeyE"]);
        assert_eq!(keys("@", a), ["AltGr+Digit0"]);
        assert_eq!(keys(",", a), ["KeyM"]);
        assert_eq!(keys("?", a), ["Shift+KeyM"]);
        assert_eq!(keys("’", a), ["Digit4"]);
        assert_eq!(keys("«", a), ["Digit3"]);
    }

    /// Espaces, retours à la ligne et tabulation.
    #[test]
    fn whitespace_keys() {
        for l in [Layout::Azerty, Layout::Qwerty, Layout::Bepo] {
            assert_eq!(keys(" ", l), ["Space"]);
            assert_eq!(keys("\u{a0}", l), ["Space"]);
            assert_eq!(keys("\n", l), ["Enter"]);
            assert_eq!(keys("\t", l), ["Tab"]);
        }
    }

    /// QWERTY : sans accents, la lettre de base est proposée.
    #[test]
    fn qwerty_keys() {
        let q = Layout::Qwerty;
        assert_eq!(keys("a", q), ["KeyA"]);
        assert_eq!(keys("A", q), ["Shift+KeyA"]);
        assert_eq!(keys("1", q), ["Digit1"]);
        assert_eq!(keys("!", q), ["Shift+Digit1"]);
        assert_eq!(keys("é", q), ["KeyE"]);
        assert_eq!(keys(";", q), ["Semicolon"]);
        assert_eq!(keys("’", q), ["Quote"]);
    }

    /// BÉPO : é, è, à, ê directs ; ç ; virgule ; majuscules par Maj.
    #[test]
    fn bepo_keys() {
        let b = Layout::Bepo;
        assert_eq!(keys("b", b), ["KeyQ"]);
        assert_eq!(keys("é", b), ["KeyW"]);
        assert_eq!(keys("É", b), ["Shift+KeyW"]);
        assert_eq!(keys("e", b), ["KeyF"]);
        assert_eq!(keys("ç", b), ["Backslash"]);
        assert_eq!(keys(",", b), ["KeyG"]);
        assert_eq!(keys("ê", b), ["IntlBackslash"]);
        assert_eq!(keys("û", b), ["KeyY", "KeyS"]);
        assert_eq!(keys("1", b), ["Shift+Digit1"]);
    }

    /// Un caractère impossible à taper ne donne aucune touche.
    #[test]
    fn untypable_character() {
        assert!(keys("漢", Layout::Azerty).is_empty());
    }

    /// Plusieurs graphèmes d'un coup, comme pour un texte.
    #[test]
    fn many_graphemes_at_once() {
        let l = layout(Layout::Azerty);
        let all = keys_for_all(&["l".into(), "ê".into()], &l);
        assert_eq!(all.len(), 2);
        assert_eq!(all[1].len(), 2);
    }

    /// Valeurs de la base pour les dispositions, dans les deux sens.
    #[test]
    fn layout_database_values() {
        for l in [Layout::Azerty, Layout::Qwerty, Layout::Bepo] {
            assert_eq!(Layout::parse(l.as_str()), l);
        }
        assert_eq!(Layout::parse("dvorak"), Layout::Azerty);
    }

    /// Chaque disposition a 5 rangées, une barre d'espace au pouce, et les doigts
    /// attendus sur la rangée de repos.
    #[test]
    fn layouts_are_complete() {
        for l in [Layout::Azerty, Layout::Qwerty, Layout::Bepo] {
            let k = layout(l);
            assert_eq!(k.rows.len(), 5, "{l:?}");
            let space = k.rows[4].iter().find(|key| key.code == "Space").unwrap();
            assert_eq!(space.finger, Finger::LeftThumb);
            let home: Vec<Finger> = k.rows[2]
                .iter()
                .filter(|key| ["KeyA", "KeyF", "KeyJ", "Semicolon"].contains(&key.code.as_str()))
                .map(|key| key.finger)
                .collect();
            assert_eq!(
                home,
                [
                    Finger::LeftPinky,
                    Finger::LeftIndex,
                    Finger::RightIndex,
                    Finger::RightPinky
                ]
            );
        }
    }
}
