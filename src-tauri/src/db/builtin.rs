//! Pack de textes français intégré (F-25, ADR 0013).
//!
//! Textes du domaine public, embarqués dans l'exécutable (`src-tauri/builtin/fr`),
//! insérés une seule fois : un texte supprimé par l'utilisateur ne revient pas.

use dactylo_core::exercise::Segmentation;
use dactylo_core::source::{BuiltinSource, TextSource};
use sqlx::SqlitePool;

use super::texts::{self, TextRecord};
use crate::error::{AppError, AppResult};

/// Version du pack ; l'augmenter ajoute les textes nouveaux aux bases existantes.
pub const PACK_VERSION: u32 = 1;

/// Clé de `app_meta` qui retient la version du pack déjà insérée.
const META_KEY: &str = "builtin_pack_version";

/// Texte du pack.
pub struct BuiltinText {
    /// Version du pack qui l'a introduit.
    pub since: u32,
    /// Titre.
    pub title: &'static str,
    /// Auteur.
    pub author: &'static str,
    /// Étiquettes.
    pub tags: &'static [&'static str],
    /// Découpage en exercices.
    pub segmentation: Segmentation,
    /// Contenu.
    pub content: &'static str,
}

/// Textes du pack, source : Wikisource (éditions du domaine public).
pub const TEXTS: &[BuiltinText] = &[
    BuiltinText {
        since: 1,
        title: "La Cigale et la Fourmi",
        author: "Jean de La Fontaine",
        tags: &["fable", "poésie"],
        segmentation: Segmentation::Whole,
        content: include_str!("../../builtin/fr/la-cigale-et-la-fourmi.txt"),
    },
    BuiltinText {
        since: 1,
        title: "Le Corbeau et le Renard",
        author: "Jean de La Fontaine",
        tags: &["fable", "poésie"],
        segmentation: Segmentation::Whole,
        content: include_str!("../../builtin/fr/le-corbeau-et-le-renard.txt"),
    },
    BuiltinText {
        since: 1,
        title: "Le Loup et l’Agneau",
        author: "Jean de La Fontaine",
        tags: &["fable", "poésie"],
        segmentation: Segmentation::Whole,
        content: include_str!("../../builtin/fr/le-loup-et-l-agneau.txt"),
    },
    BuiltinText {
        since: 1,
        title: "Demain, dès l’aube",
        author: "Victor Hugo",
        tags: &["poésie"],
        segmentation: Segmentation::Paragraphs { count: 1 },
        content: include_str!("../../builtin/fr/demain-des-l-aube.txt"),
    },
    BuiltinText {
        since: 1,
        title: "L’Albatros",
        author: "Charles Baudelaire",
        tags: &["poésie"],
        segmentation: Segmentation::Paragraphs { count: 1 },
        content: include_str!("../../builtin/fr/l-albatros.txt"),
    },
    BuiltinText {
        since: 1,
        title: "Le Tour du monde en quatre-vingts jours — chapitre I",
        author: "Jules Verne",
        tags: &["roman"],
        segmentation: Segmentation::Paragraphs { count: 4 },
        content: include_str!("../../builtin/fr/le-tour-du-monde-chapitre-1.txt"),
    },
    BuiltinText {
        since: 1,
        title: "La Parure",
        author: "Guy de Maupassant",
        tags: &["nouvelle"],
        segmentation: Segmentation::Paragraphs { count: 10 },
        content: include_str!("../../builtin/fr/la-parure.txt"),
    },
];

/// Insère les textes du pack plus récents que la version déjà installée.
pub async fn seed(pool: &SqlitePool) -> AppResult<()> {
    // Version déjà insérée (0 pour une base neuve).
    let installed: u32 =
        sqlx::query_scalar::<_, String>("SELECT value FROM app_meta WHERE key = ?")
            .bind(META_KEY)
            .fetch_optional(pool)
            .await?
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
    if installed >= PACK_VERSION {
        return Ok(());
    }
    // Seuls les textes introduits après la version installée sont ajoutés.
    for text in TEXTS.iter().filter(|t| t.since > installed) {
        let draft = BuiltinSource {
            content: text.content,
        }
        .draft()
        .map_err(|e| AppError::Invalid(e.to_string()))?;
        texts::create(
            pool,
            TextRecord {
                title: text.title.into(),
                author: Some(text.author.into()),
                content: draft.content,
                kind: draft.kind,
                source_file: None,
                pages: None,
                tags: text.tags.iter().map(|t| (*t).to_owned()).collect(),
                language: "fr".into(),
                segmentation: text.segmentation,
            },
        )
        .await?;
    }
    // Version retenue : au prochain lancement, rien n'est réinséré.
    sqlx::query("INSERT OR REPLACE INTO app_meta (key, value) VALUES (?, ?)")
        .bind(META_KEY)
        .bind(PACK_VERSION.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

/// Tests du pack intégré.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{connect_in_memory, profiles};
    use dactylo_core::exercise::exercises;
    use dactylo_core::text::{graphemes, normalize};

    /// Le pack est inséré une fois ; un texte supprimé ne revient pas.
    #[tokio::test]
    async fn pack_is_seeded_once() {
        let pool = connect_in_memory().await.unwrap();
        let profile = profiles::ensure_default(&pool).await.unwrap().id;
        // Premier lancement : tous les textes, provenance « intégré ».
        seed(&pool).await.unwrap();
        let list = texts::list(&pool, &profile).await.unwrap();
        assert_eq!(list.len(), TEXTS.len());
        assert!(
            list.iter()
                .all(|t| t.source == dactylo_core::source::SourceKind::Builtin)
        );
        // Suppression d'un texte puis nouveau lancement : il ne revient pas.
        texts::delete(&pool, &list[0].id).await.unwrap();
        seed(&pool).await.unwrap();
        assert_eq!(
            texts::list(&pool, &profile).await.unwrap().len(),
            TEXTS.len() - 1
        );
    }

    /// Chaque texte est déjà normalisé, sans caractère introuvable au clavier ni
    /// reste de mise en page, et se découpe en exercices.
    #[test]
    fn texts_are_clean() {
        for text in TEXTS {
            let content = text.content.trim_end();
            assert_eq!(normalize(content), content, "{} : pas en NFC", text.title);
            for bad in ['…', '\u{feff}', '[', '↑'] {
                assert!(!content.contains(bad), "{} contient {bad:?}", text.title);
            }
            assert!(
                !exercises(&graphemes(content), text.segmentation).is_empty(),
                "{}",
                text.title
            );
            assert!(text.since <= PACK_VERSION);
        }
    }
}
