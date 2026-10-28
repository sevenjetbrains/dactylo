//! Sources de texte derrière le trait `TextSource` (ADR 0011) : collage,
//! PDF, pack intégré ; plus tard OCR, EPUB, site web.
//!
//! Une source produit un **brouillon** (`TextDraft`) : contenu normalisé et
//! origine. L'interface l'affiche, l'utilisateur peut le corriger, puis il est
//! enregistré dans la bibliothèque.

use serde::{Deserialize, Serialize};

use crate::text::normalize;

/// Nettoyage et assemblage du texte extrait d'un PDF (F-21, F-22).
pub mod pdf;

pub use pdf::{CleanOptions, LineSelection, PageRange, PdfPage, PdfSource, SourceLine};

/// Provenance d'un texte ; valeurs identiques à la contrainte `texts.source`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum SourceKind {
    /// Copier-coller (F-20).
    Paste,
    /// Import d'un PDF (F-21).
    Pdf,
    /// Pack de textes intégré (F-25).
    Builtin,
}

impl SourceKind {
    /// Valeur stockée dans la colonne `texts.source`.
    pub fn as_str(self) -> &'static str {
        match self {
            SourceKind::Paste => "paste",
            SourceKind::Pdf => "pdf",
            SourceKind::Builtin => "builtin",
        }
    }

    /// Lit une valeur de la colonne `texts.source` ; inconnue → collage.
    pub fn from_db(value: &str) -> Self {
        match value {
            "pdf" => SourceKind::Pdf,
            "builtin" => SourceKind::Builtin,
            _ => SourceKind::Paste,
        }
    }
}

/// Brouillon produit par une source, avant enregistrement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextDraft {
    /// Contenu normalisé (NFC, fins de ligne `\n`).
    pub content: String,
    /// Provenance.
    pub kind: SourceKind,
    /// Nom du fichier d'origine (PDF).
    pub source_file: Option<String>,
    /// Pages retenues, ex. « 3-5 » (PDF).
    pub pages: Option<String>,
}

/// Échec d'une source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceError {
    /// Aucun texte : texte vide, ou PDF scanné (images seules, OCR hors V1).
    NoText,
    /// Plage de pages ou de lignes impossible.
    InvalidRange(String),
}

impl std::fmt::Display for SourceError {
    /// Message lisible, en français.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceError::NoText => write!(
                f,
                "aucun texte trouvé (un PDF scanné ne contient que des images)"
            ),
            SourceError::InvalidRange(reason) => write!(f, "sélection invalide : {reason}"),
        }
    }
}

impl std::error::Error for SourceError {}

/// Toute source de texte : elle produit un brouillon ou explique pourquoi elle ne peut pas.
pub trait TextSource {
    /// Construit le brouillon à partir de la source.
    fn draft(&self) -> Result<TextDraft, SourceError>;
}

/// Texte collé par l'utilisateur (F-20).
pub struct PasteSource {
    /// Texte tel que collé.
    pub content: String,
}

impl TextSource for PasteSource {
    /// Normalise le texte ; un texte vide est refusé.
    fn draft(&self) -> Result<TextDraft, SourceError> {
        draft_from(&self.content, SourceKind::Paste)
    }
}

/// Texte du pack intégré (F-25), déjà propre.
pub struct BuiltinSource {
    /// Contenu du fichier embarqué.
    pub content: &'static str,
}

impl TextSource for BuiltinSource {
    /// Normalise le texte embarqué.
    fn draft(&self) -> Result<TextDraft, SourceError> {
        draft_from(self.content, SourceKind::Builtin)
    }
}

/// Brouillon sans fichier ni pages ; `NoText` si le texte est vide après normalisation.
fn draft_from(content: &str, kind: SourceKind) -> Result<TextDraft, SourceError> {
    let content = normalize(content);
    if content.trim().is_empty() {
        return Err(SourceError::NoText);
    }
    Ok(TextDraft {
        content,
        kind,
        source_file: None,
        pages: None,
    })
}

/// Tests des sources simples.
#[cfg(test)]
mod tests {
    use super::*;

    /// Un texte collé est normalisé ; vide, il est refusé.
    #[test]
    fn paste_source_normalizes_and_rejects_empty_text() {
        let draft = PasteSource {
            content: "fore\u{302}t  \r\n".into(),
        }
        .draft()
        .unwrap();
        assert_eq!(draft.content, "forêt");
        assert_eq!(draft.kind, SourceKind::Paste);
        assert_eq!(draft.pages, None);
        let empty = PasteSource {
            content: " \n ".into(),
        };
        assert_eq!(empty.draft(), Err(SourceError::NoText));
    }

    /// Un texte intégré garde sa provenance.
    #[test]
    fn builtin_source_keeps_its_kind() {
        let draft = BuiltinSource {
            content: "Maître Corbeau",
        }
        .draft()
        .unwrap();
        assert_eq!(draft.kind, SourceKind::Builtin);
    }

    /// Les valeurs de provenance correspondent à la base, dans les deux sens.
    #[test]
    fn source_kind_round_trips_database_values() {
        for kind in [SourceKind::Paste, SourceKind::Pdf, SourceKind::Builtin] {
            assert_eq!(SourceKind::from_db(kind.as_str()), kind);
        }
        assert_eq!(SourceKind::from_db("inconnu"), SourceKind::Paste);
    }

    /// Les erreurs ont un message lisible.
    #[test]
    fn errors_have_readable_messages() {
        assert!(SourceError::NoText.to_string().contains("scanné"));
        assert!(
            SourceError::InvalidRange("page 9".into())
                .to_string()
                .contains("page 9")
        );
    }
}
