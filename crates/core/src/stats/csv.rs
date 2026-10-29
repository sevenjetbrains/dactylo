//! Export des sessions en CSV (F-35).
//!
//! Point-virgule comme séparateur, BOM UTF-8 et virgule décimale au choix :
//! Excel en français ouvre ainsi le fichier sans assistant d'import.
//! Les en-têtes sont fournis par l'interface, traduits (NF-08).

use crate::days::format_local;

/// Marque d'ordre des octets UTF-8 : sans elle, Excel lit le fichier en ANSI.
const UTF8_BOM: &str = "\u{feff}";

/// Une session à exporter.
#[derive(Debug, Clone, PartialEq)]
pub struct CsvSession {
    /// Début, en ms Unix.
    pub started_at: i64,
    /// Titre du texte, s'il existe encore.
    pub title: Option<String>,
    /// Mode (`copy`, `dictation`, `review`), traduit par l'interface.
    pub mode: String,
    /// État (terminée, abandonnée), traduit par l'interface.
    pub status: String,
    /// Temps actif, en ms.
    pub duration_ms: u32,
    /// MPM brut, pour une session terminée.
    pub wpm_raw: Option<f64>,
    /// MPM net.
    pub wpm_net: Option<f64>,
    /// CPM.
    pub cpm: Option<f64>,
    /// Précision, en %.
    pub accuracy: Option<f64>,
    /// Nombre de fautes relevées.
    pub errors: u32,
}

/// Format du fichier.
#[derive(Debug, Clone, PartialEq)]
pub struct CsvFormat {
    /// En-têtes des 10 colonnes, dans l'ordre de `CsvSession`.
    pub headers: Vec<String>,
    /// Virgule décimale (français) plutôt que point.
    pub decimal_comma: bool,
    /// Décalage horaire local, en minutes, pour les dates.
    pub tz_offset_min: i32,
}

/// Produit le fichier CSV : BOM, en-têtes, puis une ligne par session.
pub fn sessions_csv(sessions: &[CsvSession], format: &CsvFormat) -> String {
    let mut out = String::from(UTF8_BOM);
    push_row(&mut out, format.headers.iter().map(String::as_str));
    // Nombre décimal à une décimale ; cellule vide pour une valeur absente.
    let number = |v: Option<f64>| -> String {
        v.map(|v| {
            let s = format!("{v:.1}");
            if format.decimal_comma {
                s.replace('.', ",")
            } else {
                s
            }
        })
        .unwrap_or_default()
    };
    for s in sessions {
        let cells = [
            format_local(s.started_at, format.tz_offset_min),
            s.title.clone().unwrap_or_default(),
            s.mode.clone(),
            s.status.clone(),
            number(Some(f64::from(s.duration_ms) / 1000.0)),
            number(s.wpm_raw),
            number(s.wpm_net),
            number(s.cpm),
            number(s.accuracy),
            s.errors.to_string(),
        ];
        push_row(&mut out, cells.iter().map(String::as_str));
    }
    out
}

/// Ajoute une ligne terminée par CRLF (RFC 4180).
fn push_row<'a>(out: &mut String, cells: impl Iterator<Item = &'a str>) {
    let row: Vec<String> = cells.map(escape).collect();
    out.push_str(&row.join(";"));
    out.push_str("\r\n");
}

/// Met une cellule entre guillemets si elle contient un séparateur, un guillemet
/// ou un retour à la ligne ; les guillemets sont doublés.
fn escape(cell: &str) -> String {
    if cell.contains([';', '"', '\n', '\r']) {
        format!("\"{}\"", cell.replace('"', "\"\""))
    } else {
        cell.to_owned()
    }
}

/// Tests de l'export CSV.
#[cfg(test)]
mod tests {
    use super::*;

    /// 29 octobre 2026, 17 h 42 UTC.
    const T: i64 = 1_793_232_000_000 + 17 * 3_600_000 + 42 * 60_000;

    /// En-têtes courts pour les tests.
    fn format(decimal_comma: bool) -> CsvFormat {
        CsvFormat {
            headers: (1..=10).map(|i| format!("c{i}")).collect(),
            decimal_comma,
            tz_offset_min: 60,
        }
    }

    /// Session terminée de test.
    fn session() -> CsvSession {
        CsvSession {
            started_at: T,
            title: Some("Le Tour du monde".into()),
            mode: "Copie".into(),
            status: "Terminée".into(),
            duration_ms: 61_500,
            wpm_raw: Some(52.26),
            wpm_net: Some(50.0),
            cpm: Some(250.0),
            accuracy: Some(97.14),
            errors: 3,
        }
    }

    /// BOM, en-têtes puis une ligne, virgule décimale et heure locale.
    #[test]
    fn writes_bom_headers_and_rows() {
        let csv = sessions_csv(&[session()], &format(true));
        let expected = "\u{feff}c1;c2;c3;c4;c5;c6;c7;c8;c9;c10\r\n\
            2026-10-29 18:42;Le Tour du monde;Copie;Terminée;61,5;52,3;50,0;250,0;97,1;3\r\n";
        assert_eq!(csv, expected);
    }

    /// Point décimal au choix ; cellules vides pour une session abandonnée.
    #[test]
    fn abandoned_session_has_empty_stats() {
        let abandoned = CsvSession {
            title: None,
            status: "Abandonnée".into(),
            wpm_raw: None,
            wpm_net: None,
            cpm: None,
            accuracy: None,
            errors: 0,
            ..session()
        };
        let csv = sessions_csv(&[abandoned], &format(false));
        let line = csv.lines().nth(1).unwrap();
        assert_eq!(line, "2026-10-29 18:42;;Copie;Abandonnée;61.5;;;;;0");
    }

    /// Les séparateurs, guillemets et retours à la ligne sont protégés.
    #[test]
    fn escapes_special_cells() {
        assert_eq!(escape("simple"), "simple");
        assert_eq!(escape("a;b"), "\"a;b\"");
        assert_eq!(escape("dit \"non\""), "\"dit \"\"non\"\"\"");
        assert_eq!(escape("deux\nlignes"), "\"deux\nlignes\"");
    }

    /// Sans session, seuls le BOM et les en-têtes.
    #[test]
    fn empty_export_has_headers_only() {
        let csv = sessions_csv(&[], &format(true));
        assert_eq!(csv.lines().count(), 1);
        assert!(csv.starts_with(UTF8_BOM));
    }
}
