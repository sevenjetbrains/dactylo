//! Texte lu à voix haute, ponctuation dite en toutes lettres en option (F-13),
//! comme une dictée scolaire.

/// Nom d'un signe de ponctuation dit à voix haute, s'il en a un.
fn spoken_name(c: char, quote_open: &mut bool) -> Option<&'static str> {
    Some(match c {
        ',' => "virgule",
        '.' => "point",
        '…' => "points de suspension",
        ';' => "point-virgule",
        ':' => "deux-points",
        '?' => "point d'interrogation",
        '!' => "point d'exclamation",
        '«' | '“' => "ouvrez les guillemets",
        '»' | '”' => "fermez les guillemets",
        // Guillemet droit : alternativement ouvrant et fermant.
        '"' => {
            *quote_open = !*quote_open;
            if *quote_open {
                "ouvrez les guillemets"
            } else {
                "fermez les guillemets"
            }
        }
        '(' => "ouvrez la parenthèse",
        ')' => "fermez la parenthèse",
        '—' | '–' => "tiret",
        '\n' => "à la ligne",
        _ => return None,
    })
}

/// Texte à lire pour un segment. Sans lecture de la ponctuation, le texte tel quel
/// (la voix marque les pauses d'elle-même). Avec, chaque signe est remplacé par son
/// nom suivi d'une virgule, qui donne une courte pause ; le trait d'union et
/// l'apostrophe à l'intérieur des mots ne sont pas lus.
pub fn spoken(text: &str, read_punctuation: bool) -> String {
    if !read_punctuation {
        return text.to_owned();
    }
    // Trois points de suite valent des points de suspension.
    let text = text.replace("...", "…");
    let mut out = String::new();
    let mut quote_open = false;
    for c in text.chars() {
        match spoken_name(c, &mut quote_open) {
            Some(name) => {
                out.push(' ');
                out.push_str(name);
                out.push_str(", ");
            }
            None => out.push(c),
        }
    }
    // Espaces doublées par les remplacements ramenées à une seule.
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Tests de la ponctuation lue.
#[cfg(test)]
mod tests {
    use super::*;

    /// Sans lecture : texte inchangé.
    #[test]
    fn without_reading_text_is_unchanged() {
        assert_eq!(spoken("Oui, dit-il.", false), "Oui, dit-il.");
    }

    /// Chaque signe est dit, sauf le trait d'union et l'apostrophe internes.
    #[test]
    fn reads_punctuation_names() {
        assert_eq!(
            spoken("Oui, dit-il ; l'été.", true),
            "Oui virgule, dit-il point-virgule, l'été point,"
        );
        assert_eq!(
            spoken("« Viens ! » (vite)\nFin...", true),
            "ouvrez les guillemets, Viens point d'exclamation, fermez les guillemets, \
             ouvrez la parenthèse, vite fermez la parenthèse, à la ligne, Fin points de suspension,"
        );
    }

    /// Guillemets droits alternés ; deux-points, interrogation et tiret.
    #[test]
    fn straight_quotes_alternate() {
        assert_eq!(
            spoken("\"a\" : b ? — c", true),
            "ouvrez les guillemets, a fermez les guillemets, deux-points, b point d'interrogation, tiret, c"
        );
    }
}
