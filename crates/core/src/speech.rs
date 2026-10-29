//! Synthèse vocale derrière le trait `SpeechEngine` (F-12, ADR 0019) : voix du
//! système aujourd'hui, voix neuronales hors ligne (Piper) demain.

use serde::{Deserialize, Serialize};

/// Une voix installée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Voice {
    /// Identifiant stable, à enregistrer dans les réglages.
    pub id: String,
    /// Nom affiché (« Microsoft Hortense »).
    pub name: String,
    /// Langue BCP 47 (« fr-FR »).
    pub language: String,
}

/// Réglages d'une lecture : valeurs relatives, 1 = valeur normale du moteur.
#[derive(Debug, Clone, PartialEq)]
pub struct VoiceSettings {
    /// Voix choisie ; `None` = voix française par défaut.
    pub voice: Option<String>,
    /// Vitesse relative (0,5 à 2).
    pub rate: f64,
    /// Hauteur relative (0,5 à 2).
    pub pitch: f64,
    /// Volume, de 0 à 1.
    pub volume: f64,
}

/// Erreur d'un moteur vocal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeechError(pub String);

impl std::fmt::Display for SpeechError {
    /// « synthèse vocale : <message> ».
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "synthèse vocale : {}", self.0)
    }
}

impl std::error::Error for SpeechError {}

/// Moteur de synthèse vocale. `speak` lance la lecture et rend la main ; le moteur
/// signale la fin de lecture à part (événement de l'adaptateur), avec le numéro de
/// lecture renvoyé par `speak`.
pub trait SpeechEngine: Send + Sync {
    /// Voix installées.
    fn voices(&self) -> Result<Vec<Voice>, SpeechError>;
    /// Lit `text` avec `settings`, en interrompant toute lecture en cours ; renvoie
    /// le numéro de cette lecture, croissant.
    fn speak(&self, text: &str, settings: &VoiceSettings) -> Result<u64, SpeechError>;
    /// Interrompt la lecture en cours.
    fn stop(&self) -> Result<(), SpeechError>;
}

/// Bornes d'un paramètre propre à un moteur (vitesse, hauteur ou volume).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngineRange {
    /// Valeur minimale acceptée.
    pub min: f32,
    /// Valeur normale.
    pub normal: f32,
    /// Valeur maximale acceptée.
    pub max: f32,
}

impl EngineRange {
    /// Valeur du moteur pour un facteur relatif (1 = normale), bornée.
    pub fn scale(self, relative: f64) -> f32 {
        (self.normal * relative as f32).clamp(self.min, self.max)
    }

    /// Valeur du moteur pour une proportion de 0 à 1 (volume), bornée.
    pub fn lerp(self, fraction: f64) -> f32 {
        let f = fraction.clamp(0.0, 1.0) as f32;
        self.min + (self.max - self.min) * f
    }
}

/// Voix à utiliser : celle demandée si elle existe, sinon la première voix française,
/// sinon aucune (voix par défaut du système).
pub fn pick_voice<'a>(voices: &'a [Voice], wanted: Option<&str>) -> Option<&'a Voice> {
    wanted
        .and_then(|id| voices.iter().find(|v| v.id == id))
        .or_else(|| {
            voices
                .iter()
                .find(|v| v.language.to_lowercase().starts_with("fr"))
        })
}

/// Tests des outils communs aux moteurs.
#[cfg(test)]
mod tests {
    use super::*;

    /// Voix de test.
    fn voice(id: &str, language: &str) -> Voice {
        Voice {
            id: id.into(),
            name: id.into(),
            language: language.into(),
        }
    }

    /// Facteurs relatifs ramenés dans les bornes du moteur.
    #[test]
    fn scales_relative_values() {
        let rate = EngineRange {
            min: 0.5,
            normal: 1.0,
            max: 6.0,
        };
        assert_eq!(rate.scale(1.0), 1.0);
        assert_eq!(rate.scale(2.0), 2.0);
        assert_eq!(rate.scale(0.1), 0.5);
        let volume = EngineRange {
            min: 0.0,
            normal: 1.0,
            max: 1.0,
        };
        assert_eq!(volume.lerp(0.5), 0.5);
        assert_eq!(volume.lerp(4.0), 1.0);
    }

    /// Voix demandée, sinon française, sinon aucune.
    #[test]
    fn picks_requested_then_french_voice() {
        let voices = [
            voice("en", "en-US"),
            voice("fr1", "fr-FR"),
            voice("fr2", "fr-CA"),
        ];
        assert_eq!(pick_voice(&voices, Some("fr2")).unwrap().id, "fr2");
        assert_eq!(pick_voice(&voices, Some("absente")).unwrap().id, "fr1");
        assert_eq!(pick_voice(&voices, None).unwrap().id, "fr1");
        assert!(pick_voice(&voices[..1], None).is_none());
    }

    /// Message d'erreur lisible.
    #[test]
    fn error_message() {
        assert_eq!(
            SpeechError("aucune voix".into()).to_string(),
            "synthèse vocale : aucune voix"
        );
    }
}
