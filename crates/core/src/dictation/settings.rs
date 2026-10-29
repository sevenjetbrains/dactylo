//! Réglages de la dictée (F-11 à F-15) et des profils de dictée (F-17), avec
//! valeurs par défaut et bornes ; lecture tolérante comme les réglages de copie.

use serde::{Deserialize, Serialize};

/// Vitesse de lecture relative : de 0,5× à 2× (F-12).
pub const RATE_RANGE: (f64, f64) = (0.5, 2.0);
/// Hauteur de voix relative.
pub const PITCH_RANGE: (f64, f64) = (0.5, 2.0);
/// Volume, de 0 (muet) à 1.
pub const VOLUME_RANGE: (f64, f64) = (0.0, 1.0);
/// Taille d'un groupe de mots (F-11).
pub const GROUP_SIZE_RANGE: (u32, u32) = (2, 12);
/// Pause entre segments, de 0 à 10 s (F-12).
pub const PAUSE_RANGE_MS: (u32, u32) = (0, 10_000);

/// Découpage de la lecture (F-11) ; valeurs de `dictation_profiles.segmentation`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum SegmentMode {
    /// Un mot à la fois.
    Word,
    /// Des groupes de `group_size` mots.
    #[default]
    Group,
    /// Une phrase à la fois.
    Sentence,
    /// Un paragraphe à la fois.
    Paragraph,
}

impl SegmentMode {
    /// Valeur de la colonne `dictation_profiles.segmentation`.
    pub fn as_str(self) -> &'static str {
        match self {
            SegmentMode::Word => "word",
            SegmentMode::Group => "group",
            SegmentMode::Sentence => "sentence",
            SegmentMode::Paragraph => "paragraph",
        }
    }

    /// Lit la colonne ; une valeur inconnue vaut `Group`.
    pub fn parse(value: &str) -> Self {
        match value {
            "word" => SegmentMode::Word,
            "sentence" => SegmentMode::Sentence,
            "paragraph" => SegmentMode::Paragraph,
            _ => SegmentMode::Group,
        }
    }
}

/// Avancement d'un segment au suivant (F-15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum Advance {
    /// Le segment suivant est lu quand les mots du segment courant sont tapés.
    #[default]
    Adaptive,
    /// Le segment suivant est lu après la pause, quoi qu'il arrive.
    Automatic,
}

/// Réglages d'une dictée.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase", default)]
pub struct DictationSettings {
    /// Identifiant de la voix ; `None` = voix française par défaut du système.
    pub voice: Option<String>,
    /// Vitesse relative, 1 = normale.
    pub rate: f64,
    /// Hauteur relative, 1 = normale.
    pub pitch: f64,
    /// Volume, de 0 à 1.
    pub volume: f64,
    /// Découpage de la lecture.
    pub segmentation: SegmentMode,
    /// Mots par groupe, pour le découpage par groupe.
    pub group_size: u32,
    /// Pause entre deux segments, en ms.
    pub pause_ms: u32,
    /// Lire la ponctuation à voix haute (F-13).
    pub read_punctuation: bool,
    /// Avancement adaptatif ou automatique (F-15).
    pub advance: Advance,
    /// Répétitions autorisées par segment ; `None` = illimité (F-14).
    pub max_repeats: Option<u32>,
    /// Tolère les variantes typographiques à la correction (’ = ', « » = ").
    pub lenient_punctuation: bool,
}

impl Default for DictationSettings {
    /// Valeurs par défaut du cahier des charges : groupes de 4 mots, 1 s de pause,
    /// répétitions illimitées.
    fn default() -> Self {
        Self {
            voice: None,
            rate: 1.0,
            pitch: 1.0,
            volume: 1.0,
            segmentation: SegmentMode::Group,
            group_size: 4,
            pause_ms: 1_000,
            read_punctuation: false,
            advance: Advance::Adaptive,
            max_repeats: None,
            lenient_punctuation: true,
        }
    }
}

/// Ramène un nombre dans ses bornes ; une valeur non finie prend `fallback`.
fn clamp(value: f64, range: (f64, f64), fallback: f64) -> f64 {
    if value.is_finite() {
        value.clamp(range.0, range.1)
    } else {
        fallback
    }
}

impl DictationSettings {
    /// Ramène chaque valeur dans ses bornes.
    pub fn sanitized(self) -> Self {
        let d = Self::default();
        Self {
            rate: clamp(self.rate, RATE_RANGE, d.rate),
            pitch: clamp(self.pitch, PITCH_RANGE, d.pitch),
            volume: clamp(self.volume, VOLUME_RANGE, d.volume),
            group_size: self
                .group_size
                .clamp(GROUP_SIZE_RANGE.0, GROUP_SIZE_RANGE.1),
            pause_ms: self.pause_ms.clamp(PAUSE_RANGE_MS.0, PAUSE_RANGE_MS.1),
            // Une voix vide vaut « par défaut ».
            voice: self.voice.filter(|v| !v.trim().is_empty()),
            ..self
        }
    }

    /// Lit des réglages JSON ; absent ou illisible → défauts.
    pub fn from_json(value: Option<&serde_json::Value>) -> Self {
        value
            .and_then(|v| serde_json::from_value::<Self>(v.clone()).ok())
            .unwrap_or_default()
            .sanitized()
    }

    /// Une répétition de plus est-elle permise après `used` répétitions ?
    pub fn can_repeat(&self, used: u32) -> bool {
        self.max_repeats.is_none_or(|max| used < max)
    }
}

/// Tests des réglages de dictée.
#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// Défauts du cahier des charges.
    #[test]
    fn defaults() {
        let s = DictationSettings::default();
        assert_eq!((s.segmentation, s.group_size), (SegmentMode::Group, 4));
        assert_eq!(s.max_repeats, None);
        assert!(s.can_repeat(1_000));
    }

    /// Bornes : vitesse, pause, groupe, volume, valeurs non finies, voix vide.
    #[test]
    fn values_are_bounded() {
        let s = DictationSettings::from_json(Some(&json!({
            "rate": 9, "pause_ms": 1, "pauseMs": 60000, "groupSize": 1, "volume": -2, "voice": " "
        })));
        assert_eq!(s.rate, 2.0);
        assert_eq!(s.pause_ms, 10_000);
        assert_eq!(s.group_size, 2);
        assert_eq!(s.volume, 0.0);
        assert_eq!(s.voice, None);
        let nan = DictationSettings {
            pitch: f64::NAN,
            ..DictationSettings::default()
        };
        assert_eq!(nan.sanitized().pitch, 1.0);
    }

    /// JSON illisible ou absent : défauts.
    #[test]
    fn unreadable_json_gives_defaults() {
        assert_eq!(
            DictationSettings::from_json(None),
            DictationSettings::default()
        );
        assert_eq!(
            DictationSettings::from_json(Some(&json!({ "rate": "vite" }))),
            DictationSettings::default()
        );
    }

    /// Limite de répétitions (F-14).
    #[test]
    fn repeat_limit() {
        let s = DictationSettings {
            max_repeats: Some(2),
            ..DictationSettings::default()
        };
        assert!(s.can_repeat(1));
        assert!(!s.can_repeat(2));
    }

    /// Valeurs de la base pour le découpage, dans les deux sens.
    #[test]
    fn segment_mode_database_values() {
        for m in [
            SegmentMode::Word,
            SegmentMode::Group,
            SegmentMode::Sentence,
            SegmentMode::Paragraph,
        ] {
            assert_eq!(SegmentMode::parse(m.as_str()), m);
        }
        assert_eq!(SegmentMode::parse("?"), SegmentMode::Group);
    }
}
