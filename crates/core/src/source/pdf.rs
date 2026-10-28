//! Import d'un PDF (F-21) et nettoyage du texte extrait (F-22).
//!
//! pdf.js extrait côté interface les lignes de chaque page ; tout le reste est
//! ici : choix des pages et des lignes, suppression des numéros de page et des
//! en-têtes répétés, recollage des césures, reconstitution des paragraphes,
//! normalisation des espaces et des apostrophes.

use serde::{Deserialize, Serialize};

use super::{SourceError, SourceKind, TextDraft, TextSource};
use crate::text::normalize;

/// Lignes d'une page, telles qu'extraites par pdf.js.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PdfPage {
    /// Numéro de la page dans le document, à partir de 1.
    pub number: u32,
    /// Lignes de texte, de haut en bas.
    pub lines: Vec<String>,
}

/// Plage de pages, bornes comprises (ex. 3 à 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PageRange {
    /// Première page, à partir de 1.
    pub first: u32,
    /// Dernière page, comprise.
    pub last: u32,
}

impl PageRange {
    /// Forme stockée dans `texts.pages` : « 3 » ou « 3-5 ».
    pub fn label(&self) -> String {
        if self.first == self.last {
            self.first.to_string()
        } else {
            format!("{}-{}", self.first, self.last)
        }
    }

    /// La page `number` est-elle dans la plage ?
    pub fn contains(&self, number: u32) -> bool {
        (self.first..=self.last).contains(&number)
    }
}

/// Lignes retenues après nettoyage, bornes comprises, numérotées à partir de 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct LineSelection {
    /// Première ligne retenue.
    pub first: u32,
    /// Dernière ligne retenue, comprise.
    pub last: u32,
}

/// Étapes du nettoyage, toutes actives par défaut ; l'utilisateur peut en couper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase", default)]
pub struct CleanOptions {
    /// Supprimer les numéros de page en haut ou en bas de page.
    pub remove_page_numbers: bool,
    /// Supprimer les en-têtes et pieds de page répétés.
    pub remove_headers: bool,
    /// Recoller les mots coupés en fin de ligne (« exem- / ple »).
    pub join_hyphens: bool,
    /// Reconstituer les paragraphes au lieu de garder chaque ligne du PDF.
    pub reflow: bool,
    /// Espaces multiples ou insécables → une espace ; apostrophes courbes → '.
    pub normalize_spacing: bool,
}

impl Default for CleanOptions {
    /// Tout le nettoyage est actif.
    fn default() -> Self {
        Self {
            remove_page_numbers: true,
            remove_headers: true,
            join_hyphens: true,
            reflow: true,
            normalize_spacing: true,
        }
    }
}

/// Ligne nettoyée, avec sa page d'origine (aperçu et choix des lignes).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SourceLine {
    /// Page d'où vient la ligne.
    pub page: u32,
    /// Texte de la ligne.
    pub text: String,
}

/// PDF à importer : pages extraites, plage, lignes et options de nettoyage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PdfSource {
    /// Nom du fichier, gardé comme origine du texte.
    pub file_name: String,
    /// Pages extraites : celles de la plage, et si possible quelques voisines,
    /// qui aident à reconnaître les en-têtes répétés.
    pub pages: Vec<PdfPage>,
    /// Pages retenues.
    pub range: PageRange,
    /// Lignes retenues parmi celles des pages ; toutes si absent.
    pub lines: Option<LineSelection>,
    /// Étapes du nettoyage.
    #[serde(default)]
    pub options: CleanOptions,
}

impl PdfSource {
    /// Lignes nettoyées des pages de la plage, avant le choix des lignes.
    pub fn source_lines(&self) -> Result<Vec<SourceLine>, SourceError> {
        // La plage doit être croissante et commencer à la page 1 au plus tôt.
        let range = self.range;
        if range.first == 0 || range.first > range.last {
            return Err(SourceError::InvalidRange(format!(
                "pages {}",
                range.label()
            )));
        }
        // Chaque page de la plage doit avoir été extraite.
        let in_range = self
            .pages
            .iter()
            .filter(|p| range.contains(p.number))
            .count();
        if in_range != (range.last - range.first + 1) as usize {
            return Err(SourceError::InvalidRange(format!(
                "pages {} absentes du document",
                range.label()
            )));
        }
        // Le nettoyage voit toutes les pages fournies : un en-tête de page paire
        // n'apparaît qu'une fois sur les pages 3 à 5, mais plusieurs fois autour.
        let all: Vec<&PdfPage> = self.pages.iter().collect();
        let lines: Vec<SourceLine> = page_lines(&all, self.options)
            .into_iter()
            .filter(|l| range.contains(l.page))
            .collect();
        // Sans aucune ligne de texte, le PDF est probablement scanné.
        if lines.is_empty() {
            return Err(SourceError::NoText);
        }
        Ok(lines)
    }

    /// Textes des lignes retenues par la sélection.
    fn selected(&self, lines: &[SourceLine]) -> Result<Vec<String>, SourceError> {
        // Sans sélection, toutes les lignes ; sinon, des bornes valides sont exigées.
        let (first, last) = match self.lines {
            None => (1, lines.len()),
            Some(sel) => (sel.first as usize, sel.last as usize),
        };
        if first == 0 || first > last || last > lines.len() {
            return Err(SourceError::InvalidRange(format!(
                "lignes {first} à {last} sur {}",
                lines.len()
            )));
        }
        Ok(lines[first - 1..last]
            .iter()
            .map(|l| l.text.clone())
            .collect())
    }

    /// Aperçu complet : lignes des pages (pour choisir) et brouillon assemblé.
    pub fn preview(&self) -> Result<(Vec<SourceLine>, TextDraft), SourceError> {
        let lines = self.source_lines()?;
        let content = assemble(&self.selected(&lines)?, self.options);
        // Une sélection faite uniquement de lignes vidées par le nettoyage ne donne rien.
        if content.is_empty() {
            return Err(SourceError::NoText);
        }
        let draft = TextDraft {
            content,
            kind: SourceKind::Pdf,
            source_file: Some(self.file_name.clone()),
            pages: Some(self.range.label()),
        };
        Ok((lines, draft))
    }
}

impl TextSource for PdfSource {
    /// Brouillon des lignes retenues, nettoyé et assemblé.
    fn draft(&self) -> Result<TextDraft, SourceError> {
        self.preview().map(|(_, draft)| draft)
    }
}

/// Lignes des pages, sans lignes vides, numéros de page ni en-têtes répétés.
pub fn page_lines(pages: &[&PdfPage], options: CleanOptions) -> Vec<SourceLine> {
    // Lignes de chaque page, espaces resserrés, lignes vides retirées.
    let mut per_page: Vec<(u32, Vec<String>)> = pages
        .iter()
        .map(|p| {
            let lines = p
                .lines
                .iter()
                .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|l| !l.is_empty())
                .collect();
            (p.number, lines)
        })
        .collect();

    // Numéros de page : seulement parmi les deux premières et deux dernières lignes.
    if options.remove_page_numbers {
        for (_, lines) in &mut per_page {
            let n = lines.len();
            let mut i = 0;
            lines.retain(|line| {
                let edge = i < 2 || i + 2 >= n;
                i += 1;
                !(edge && is_page_number(line))
            });
        }
    }

    // En-têtes et pieds de page : première ou dernière ligne répétée sur au moins
    // deux pages et sur au moins la moitié des pages (chiffres ignorés).
    if options.remove_headers && per_page.len() >= 2 {
        let edges = |lines: &Vec<String>| -> Vec<String> {
            let mut keys: Vec<String> = [lines.first(), lines.last()]
                .into_iter()
                .flatten()
                .map(|l| header_key(l))
                .filter(|k| !k.is_empty())
                .collect();
            keys.dedup();
            keys
        };
        let all: Vec<String> = per_page.iter().flat_map(|(_, l)| edges(l)).collect();
        let pages_count = per_page.len();
        let repeated = |key: &str| {
            let count = all.iter().filter(|k| *k == key).count();
            count >= 2 && count * 2 >= pages_count
        };
        for (_, lines) in &mut per_page {
            // Seules la première et la dernière ligne peuvent être retirées.
            if lines.last().is_some_and(|l| repeated(&header_key(l))) {
                lines.pop();
            }
            if lines.first().is_some_and(|l| repeated(&header_key(l))) {
                lines.remove(0);
            }
        }
    }

    // Aplatissement, en gardant la page de chaque ligne.
    per_page
        .into_iter()
        .flat_map(|(page, lines)| lines.into_iter().map(move |text| SourceLine { page, text }))
        .collect()
}

/// Une ligne seule de type « 12 », « - 12 - », « Page 12 », « 12/40 » ou « 12 sur 40 ».
fn is_page_number(line: &str) -> bool {
    // Tirets décoratifs et mot « page » retirés.
    let lower = line.to_lowercase();
    let trimmed = lower.trim_matches(|c: char| c.is_whitespace() || "-–—".contains(c));
    let rest = trimmed
        .strip_prefix("page")
        .or_else(|| trimmed.strip_prefix("p."))
        .unwrap_or(trimmed);
    // Restent un nombre, deux nombres séparés par « / », ou « n sur m ». Trois chiffres
    // au plus : une ligne « 1872 » est plus souvent une année qu'une page.
    let tokens: Vec<&str> = rest
        .split(|c: char| c.is_whitespace() || c == '/')
        .filter(|t| !t.is_empty())
        .collect();
    let number = |t: &str| (1..=3).contains(&t.len()) && t.chars().all(|c| c.is_ascii_digit());
    match tokens.as_slice() {
        [n] => number(n),
        [a, b] => number(a) && number(b),
        [a, word, b] => number(a) && ["sur", "of"].contains(word) && number(b),
        _ => false,
    }
}

/// Clé de comparaison des en-têtes : minuscules, sans chiffres, espaces resserrés.
fn header_key(line: &str) -> String {
    line.to_lowercase()
        .chars()
        .filter(|c| !c.is_ascii_digit())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Assemble des lignes en texte : césures recollées, paragraphes reconstitués.
pub fn assemble(lines: &[String], options: CleanOptions) -> String {
    // Espaces et apostrophes normalisés, lignes vides retirées.
    let tidy = lines
        .iter()
        .map(|l| tidy_line(l, options.normalize_spacing))
        .filter(|l| !l.is_empty());

    // Césures : « exem- » suivi de « ple » donne « exemple ».
    let mut joined: Vec<String> = Vec::new();
    for line in tidy {
        let hyphenated = options.join_hyphens
            && joined.last().is_some_and(|prev| ends_with_hyphen(prev))
            && starts_lowercase(&line);
        match joined.last_mut() {
            Some(prev) if hyphenated => {
                prev.pop();
                prev.push_str(&line);
            }
            _ => joined.push(line),
        }
    }

    // Sans reconstitution, chaque ligne du PDF reste une ligne.
    if !options.reflow {
        return normalize(&joined.join("\n"));
    }

    // Paragraphes : une ligne nettement plus courte que les autres et finie par une
    // ponctuation de fin clôt le paragraphe ; un tiret de dialogue en ouvre un.
    let longest = joined.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let mut paragraphs: Vec<String> = Vec::new();
    let mut current = String::new();
    for line in joined {
        if starts_dialogue(&line) && !current.is_empty() {
            paragraphs.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(&line);
        if ends_paragraph(&line, longest) {
            paragraphs.push(std::mem::take(&mut current));
        }
    }
    // Le dernier paragraphe n'a pas forcément de ponctuation finale.
    if !current.is_empty() {
        paragraphs.push(current);
    }
    normalize(&paragraphs.join("\n"))
}

/// Resserre les espaces et, si demandé, remplace les apostrophes courbes par '.
fn tidy_line(line: &str, normalize_spacing: bool) -> String {
    if !normalize_spacing {
        return line.trim().to_owned();
    }
    // `split_whitespace` reconnaît aussi les espaces insécables.
    line.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .map(|c| match c {
            '\u{2019}' | '\u{2018}' | '\u{02bc}' => '\'',
            other => other,
        })
        .collect()
}

/// La ligne finit-elle par un trait d'union (ou un tiret conditionnel) collé à une lettre ?
fn ends_with_hyphen(line: &str) -> bool {
    let mut chars = line.chars().rev();
    matches!(chars.next(), Some('-' | '\u{2010}' | '\u{ad}'))
        && chars.next().is_some_and(char::is_alphabetic)
}

/// La ligne commence-t-elle par une minuscule (suite d'un mot coupé) ?
fn starts_lowercase(line: &str) -> bool {
    line.chars().next().is_some_and(char::is_lowercase)
}

/// La ligne ouvre-t-elle une réplique de dialogue ?
fn starts_dialogue(line: &str) -> bool {
    line.starts_with(['—', '–'])
}

/// La ligne clôt-elle un paragraphe ? Ponctuation finale et longueur < 80 % de la plus longue.
fn ends_paragraph(line: &str, longest: usize) -> bool {
    let terminal = line.chars().last().is_some_and(|c| ".!?…:»\"".contains(c));
    terminal && line.chars().count() * 5 < longest * 4
}

/// Tests du nettoyage PDF.
#[cfg(test)]
mod tests {
    use super::*;

    /// Construit une page à partir de lignes.
    fn page(number: u32, lines: &[&str]) -> PdfPage {
        PdfPage {
            number,
            lines: lines.iter().map(|l| (*l).to_owned()).collect(),
        }
    }

    /// Lignes en chaînes possédées.
    fn owned(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|l| (*l).to_owned()).collect()
    }

    /// Textes des lignes, pour comparer simplement.
    fn texts(lines: &[SourceLine]) -> Vec<&str> {
        lines.iter().map(|l| l.text.as_str()).collect()
    }

    /// Document de six pages : en-têtes alternés (pages paires / impaires),
    /// numéros de page en bas et une césure par page.
    fn book() -> Vec<PdfPage> {
        let words = ["un", "deux", "trois", "quatre", "cinq", "six"];
        (1..=6)
            .map(|n| {
                let header = if n % 2 == 0 {
                    "LE TOUR DU MONDE EN 80 JOURS"
                } else {
                    "CHAPITRE I"
                };
                let word = words[n as usize - 1];
                PdfPage {
                    number: n,
                    lines: vec![
                        header.to_owned(),
                        format!("Texte de la page {word}, avec un exem-"),
                        format!("ple de césure numéro {word}."),
                        format!("{n}"),
                    ],
                }
            })
            .collect()
    }

    /// Numéros de page reconnus sous leurs formes courantes ; le texte ordinaire non.
    #[test]
    fn page_numbers_are_recognized() {
        for line in [
            "12",
            "- 12 -",
            "— 3 —",
            "Page 7",
            "p. 7",
            "12/40",
            "12 sur 40",
            "3 of 9",
        ] {
            assert!(is_page_number(line), "{line}");
        }
        for line in ["1872", "En 1872.", "12 heures", "Chapitre 3", "12345", ""] {
            assert!(!is_page_number(line), "{line}");
        }
    }

    /// Fin de parcours : pages 3 à 5 d'un document, nettoyées (critère de la phase 2).
    #[test]
    fn pages_three_to_five_are_extracted_and_cleaned() {
        let source = PdfSource {
            file_name: "verne.pdf".into(),
            pages: book(),
            range: PageRange { first: 3, last: 5 },
            lines: None,
            options: CleanOptions::default(),
        };
        // En-têtes alternés et numéros de page retirés ; chaque ligne garde sa page.
        let (lines, draft) = source.preview().unwrap();
        assert_eq!(
            texts(&lines),
            [
                "Texte de la page trois, avec un exem-",
                "ple de césure numéro trois.",
                "Texte de la page quatre, avec un exem-",
                "ple de césure numéro quatre.",
                "Texte de la page cinq, avec un exem-",
                "ple de césure numéro cinq.",
            ]
        );
        assert_eq!(lines[2].page, 4);
        // Césures recollées ; les lignes, de longueur voisine, forment un seul paragraphe.
        assert_eq!(
            draft.content,
            "Texte de la page trois, avec un exemple de césure numéro trois. \
             Texte de la page quatre, avec un exemple de césure numéro quatre. \
             Texte de la page cinq, avec un exemple de césure numéro cinq."
        );
        assert_eq!(draft.kind, SourceKind::Pdf);
        assert_eq!(draft.pages.as_deref(), Some("3-5"));
        assert_eq!(draft.source_file.as_deref(), Some("verne.pdf"));
    }

    /// Les lignes de début et de fin choisies bornent le texte.
    #[test]
    fn line_selection_keeps_only_chosen_lines() {
        let source = PdfSource {
            file_name: "verne.pdf".into(),
            pages: book(),
            range: PageRange { first: 3, last: 4 },
            lines: Some(LineSelection { first: 2, last: 3 }),
            options: CleanOptions::default(),
        };
        // La ligne 2, courte et finie par un point, clôt un paragraphe.
        assert_eq!(
            source.draft().unwrap().content,
            "ple de césure numéro trois.\nTexte de la page quatre, avec un exem-"
        );
    }

    /// Plages de pages ou de lignes impossibles.
    #[test]
    fn invalid_ranges_are_rejected() {
        let mut source = PdfSource {
            file_name: "a.pdf".into(),
            pages: book(),
            range: PageRange { first: 5, last: 3 },
            lines: None,
            options: CleanOptions::default(),
        };
        // Plage à l'envers, page 0, pages absentes.
        assert!(matches!(source.draft(), Err(SourceError::InvalidRange(_))));
        source.range = PageRange { first: 0, last: 1 };
        assert!(matches!(source.draft(), Err(SourceError::InvalidRange(_))));
        source.range = PageRange { first: 6, last: 8 };
        assert!(matches!(source.draft(), Err(SourceError::InvalidRange(_))));
        // Lignes au-delà de celles disponibles.
        source.range = PageRange { first: 1, last: 1 };
        source.lines = Some(LineSelection { first: 1, last: 9 });
        assert!(matches!(source.draft(), Err(SourceError::InvalidRange(_))));
        source.lines = Some(LineSelection { first: 0, last: 1 });
        assert!(matches!(source.draft(), Err(SourceError::InvalidRange(_))));
    }

    /// Un PDF scanné (pages sans texte) donne un message clair.
    #[test]
    fn scanned_pdf_has_no_text() {
        let source = PdfSource {
            file_name: "scan.pdf".into(),
            pages: vec![page(1, &[]), page(2, &["   "])],
            range: PageRange { first: 1, last: 2 },
            lines: None,
            options: CleanOptions::default(),
        };
        assert_eq!(source.draft(), Err(SourceError::NoText));
    }

    /// Chaque étape du nettoyage peut être coupée.
    #[test]
    fn cleaning_steps_can_be_disabled() {
        let pages = book();
        let selected: Vec<&PdfPage> = pages.iter().filter(|p| p.number <= 2).collect();
        let none = CleanOptions {
            remove_page_numbers: false,
            remove_headers: false,
            join_hyphens: false,
            reflow: false,
            normalize_spacing: false,
        };
        // Sans nettoyage, en-têtes et numéros restent.
        let raw = page_lines(&selected, none);
        assert_eq!(raw.len(), 8);
        assert_eq!(raw[0].text, "CHAPITRE I");
        // Sans recollage ni reconstitution, les lignes restent telles quelles.
        assert_eq!(
            assemble(&owned(&["un exem-", "ple"]), none),
            "un exem-\nple"
        );
    }

    /// Une page seule n'a pas d'en-tête « répété » : sa première ligne est gardée.
    #[test]
    fn single_page_keeps_its_first_line() {
        let pages = [page(1, &["Titre", "Texte."])];
        let refs: Vec<&PdfPage> = pages.iter().collect();
        assert_eq!(
            texts(&page_lines(&refs, CleanOptions::default())),
            ["Titre", "Texte."]
        );
    }

    /// Paragraphes : ligne courte finale ou tiret de dialogue.
    #[test]
    fn paragraphs_are_rebuilt() {
        let lines = owned(&[
            "Phileas Fogg était un homme fort exact, qui ne",
            "se pressait jamais.",
            "— Passepartout, dit-il, nous partons dans dix",
            "minutes.",
            "Le domestique obéit sans rien dire de plus",
        ]);
        assert_eq!(
            assemble(&lines, CleanOptions::default()),
            "Phileas Fogg était un homme fort exact, qui ne se pressait jamais.\n\
             — Passepartout, dit-il, nous partons dans dix minutes.\n\
             Le domestique obéit sans rien dire de plus"
        );
    }

    /// Une césure n'est recollée que devant une minuscule (pas devant un nom propre).
    #[test]
    fn hyphen_is_kept_before_a_capital() {
        let lines = owned(&["Saville-", "Row et Burlington"]);
        let options = CleanOptions {
            reflow: false,
            ..CleanOptions::default()
        };
        assert_eq!(assemble(&lines, options), "Saville-\nRow et Burlington");
    }

    /// Espaces multiples ou insécables resserrés ; apostrophes courbes redressées.
    #[test]
    fn spacing_and_apostrophes_are_normalized() {
        let lines = owned(&["l\u{2019}année  1872\u{a0}:"]);
        assert_eq!(assemble(&lines, CleanOptions::default()), "l'année 1872 :");
    }

    /// Libellé de la plage et appartenance d'une page.
    #[test]
    fn page_range_label_and_contains() {
        assert_eq!(PageRange { first: 3, last: 5 }.label(), "3-5");
        assert_eq!(PageRange { first: 4, last: 4 }.label(), "4");
        assert!(PageRange { first: 3, last: 5 }.contains(5));
        assert!(!PageRange { first: 3, last: 5 }.contains(6));
    }
}
