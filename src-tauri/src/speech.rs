//! Moteurs vocaux (ADR 0019) : `TtsEngine` sur la crate `tts` (voix du système,
//! hors ligne), et `FakeSpeechEngine` pour les tests.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use dactylo_core::speech::{
    EngineRange, SpeechEngine, SpeechError, Voice, VoiceSettings, pick_voice,
};

/// Rappel appelé à la fin d'une lecture, avec son numéro.
pub type OnFinished = Arc<dyn Fn(u64) + Send + Sync>;

/// Ordre envoyé au fil qui possède le moteur `tts`.
enum Command {
    /// Liste des voix.
    Voices(Sender<Result<Vec<Voice>, SpeechError>>),
    /// Lecture d'un texte, avec le numéro qui lui est attribué.
    Speak(String, VoiceSettings, u64, Sender<Result<(), SpeechError>>),
    /// Arrêt de la lecture en cours.
    Stop(Sender<Result<(), SpeechError>>),
}

/// Erreur de la crate `tts` traduite pour le cœur.
fn speech_error(e: impl std::fmt::Display) -> SpeechError {
    SpeechError(e.to_string())
}

/// Voix de la crate `tts` traduite pour le cœur.
fn to_voice(v: &tts::Voice) -> Voice {
    Voice {
        id: v.id(),
        name: v.name(),
        language: v.language().to_string(),
    }
}

/// Moteur `tts`. La crate n'est pas sûre entre fils (elle repose sur `Rc`) : un fil
/// dédié la possède et reçoit les ordres par canal.
pub struct TtsEngine {
    /// Canal vers le fil du moteur.
    commands: Mutex<Sender<Command>>,
    /// Dernier numéro de lecture attribué.
    next: AtomicU64,
}

impl TtsEngine {
    /// Démarre le moteur ; `on_finished` est appelé à la fin de chaque lecture.
    pub fn start(on_finished: OnFinished) -> Result<Self, SpeechError> {
        let (tx, rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        thread::Builder::new()
            .name("tts".into())
            .spawn(move || run(rx, ready_tx, on_finished))
            .map_err(speech_error)?;
        // Le fil confirme que le moteur a pu être créé.
        ready_rx.recv().map_err(speech_error)??;
        Ok(Self {
            commands: Mutex::new(tx),
            next: AtomicU64::new(0),
        })
    }

    /// Envoie un ordre au fil du moteur et attend sa réponse.
    fn ask<T>(
        &self,
        make: impl FnOnce(Sender<Result<T, SpeechError>>) -> Command,
    ) -> Result<T, SpeechError> {
        let (tx, rx) = mpsc::channel();
        self.commands
            .lock()
            .map_err(speech_error)?
            .send(make(tx))
            .map_err(speech_error)?;
        rx.recv().map_err(speech_error)?
    }
}

impl SpeechEngine for TtsEngine {
    /// Voix installées sur le système.
    fn voices(&self) -> Result<Vec<Voice>, SpeechError> {
        self.ask(Command::Voices)
    }

    /// Lecture d'un texte ; le numéro est attribué ici, avant l'envoi.
    fn speak(&self, text: &str, settings: &VoiceSettings) -> Result<u64, SpeechError> {
        let id = self.next.fetch_add(1, Ordering::SeqCst) + 1;
        self.ask(|tx| Command::Speak(text.to_owned(), settings.clone(), id, tx))?;
        Ok(id)
    }

    /// Arrêt de la lecture en cours.
    fn stop(&self) -> Result<(), SpeechError> {
        self.ask(Command::Stop)
    }
}

/// Boucle du fil du moteur : crée le moteur, branche le rappel de fin, puis exécute
/// les ordres jusqu'à la fermeture du canal.
fn run(
    commands: Receiver<Command>,
    ready: Sender<Result<(), SpeechError>>,
    on_finished: OnFinished,
) {
    let mut engine = match tts::Tts::default() {
        Ok(engine) => engine,
        Err(e) => {
            let _ = ready.send(Err(speech_error(e)));
            return;
        }
    };
    // Numéro de la lecture en cours, lu par le rappel de fin.
    let current = Arc::new(AtomicU64::new(0));
    let ended = current.clone();
    let callback = on_finished.clone();
    // Sans rappels (moteur limité), l'avancement automatique ne pourra pas suivre.
    let _ = engine.on_utterance_end(Some(Box::new(move |_| {
        callback(ended.load(Ordering::SeqCst));
    })));
    let _ = ready.send(Ok(()));

    for command in commands {
        match command {
            Command::Voices(reply) => {
                let voices = engine
                    .voices()
                    .map(|vs| vs.iter().map(to_voice).collect())
                    .map_err(speech_error);
                let _ = reply.send(voices);
            }
            Command::Speak(text, settings, id, reply) => {
                current.store(id, Ordering::SeqCst);
                let _ = reply.send(speak(&mut engine, &text, &settings));
            }
            Command::Stop(reply) => {
                let _ = reply.send(engine.stop().map(|_| ()).map_err(speech_error));
            }
        }
    }
}

/// Règle la voix, la vitesse, la hauteur et le volume, puis lit le texte.
fn speak(engine: &mut tts::Tts, text: &str, settings: &VoiceSettings) -> Result<(), SpeechError> {
    // Voix demandée, sinon française ; une erreur de voix n'empêche pas la lecture.
    if let Ok(voices) = engine.voices() {
        let ours: Vec<Voice> = voices.iter().map(to_voice).collect();
        if let Some(chosen) = pick_voice(&ours, settings.voice.as_deref())
            && let Some(v) = voices.iter().find(|v| v.id() == chosen.id)
        {
            let _ = engine.set_voice(v);
        }
    }
    // Valeurs relatives converties dans les bornes de ce moteur.
    let rate = EngineRange {
        min: engine.min_rate(),
        normal: engine.normal_rate(),
        max: engine.max_rate(),
    };
    let pitch = EngineRange {
        min: engine.min_pitch(),
        normal: engine.normal_pitch(),
        max: engine.max_pitch(),
    };
    let volume = EngineRange {
        min: engine.min_volume(),
        normal: engine.normal_volume(),
        max: engine.max_volume(),
    };
    let _ = engine.set_rate(rate.scale(settings.rate));
    let _ = engine.set_pitch(pitch.scale(settings.pitch));
    let _ = engine.set_volume(volume.lerp(settings.volume));
    engine.speak(text, true).map(|_| ()).map_err(speech_error)
}

/// Moteur absent (aucune synthèse disponible) : toute lecture échoue avec un message.
pub struct UnavailableEngine(
    /// Raison de l'indisponibilité.
    pub String,
);

impl SpeechEngine for UnavailableEngine {
    /// Aucune voix.
    fn voices(&self) -> Result<Vec<Voice>, SpeechError> {
        Ok(Vec::new())
    }

    /// Lecture impossible.
    fn speak(&self, _: &str, _: &VoiceSettings) -> Result<u64, SpeechError> {
        Err(SpeechError(self.0.clone()))
    }

    /// Rien à arrêter.
    fn stop(&self) -> Result<(), SpeechError> {
        Ok(())
    }
}

/// Moteur de test : mémorise les textes lus, sans son.
#[derive(Default)]
pub struct FakeSpeechEngine {
    /// Textes lus, dans l'ordre, avec leurs réglages.
    pub spoken: Mutex<Vec<(String, VoiceSettings)>>,
    /// Nombre d'arrêts demandés.
    pub stops: AtomicU64,
}

impl SpeechEngine for FakeSpeechEngine {
    /// Deux voix, dont une française.
    fn voices(&self) -> Result<Vec<Voice>, SpeechError> {
        Ok(vec![
            Voice {
                id: "fake-en".into(),
                name: "Fake English".into(),
                language: "en-US".into(),
            },
            Voice {
                id: "fake-fr".into(),
                name: "Fausse voix".into(),
                language: "fr-FR".into(),
            },
        ])
    }

    /// Mémorise le texte ; le numéro est le rang de la lecture.
    fn speak(&self, text: &str, settings: &VoiceSettings) -> Result<u64, SpeechError> {
        let mut spoken = self.spoken.lock().map_err(speech_error)?;
        spoken.push((text.to_owned(), settings.clone()));
        Ok(spoken.len() as u64)
    }

    /// Compte l'arrêt.
    fn stop(&self) -> Result<(), SpeechError> {
        self.stops.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

impl FakeSpeechEngine {
    /// Textes lus jusqu'ici.
    pub fn texts(&self) -> Vec<String> {
        self.spoken
            .lock()
            .map(|s| s.iter().map(|(t, _)| t.clone()).collect())
            .unwrap_or_default()
    }
}

/// Tests des moteurs sans son.
#[cfg(test)]
mod tests {
    use super::*;

    /// Réglages de lecture par défaut.
    fn settings() -> VoiceSettings {
        VoiceSettings {
            voice: None,
            rate: 1.0,
            pitch: 1.0,
            volume: 1.0,
        }
    }

    /// Le moteur factice numérote les lectures et compte les arrêts.
    #[test]
    fn fake_engine_records_speech() {
        let engine = FakeSpeechEngine::default();
        assert_eq!(engine.speak("un", &settings()).unwrap(), 1);
        assert_eq!(engine.speak("deux", &settings()).unwrap(), 2);
        engine.stop().unwrap();
        assert_eq!(engine.texts(), ["un", "deux"]);
        assert_eq!(engine.stops.load(Ordering::SeqCst), 1);
        assert_eq!(engine.voices().unwrap().len(), 2);
    }

    /// Vrai moteur du système : voix françaises, lecture muette et rappel de fin.
    /// Ignoré par défaut (dépend des voix installées) : `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn system_engine_speaks_and_reports_end() {
        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        let engine = TtsEngine::start(Arc::new(move |id| {
            let _ = tx.lock().unwrap().send(id);
        }))
        .unwrap();
        let voices = engine.voices().unwrap();
        assert!(
            voices.iter().any(|v| v.language.starts_with("fr")),
            "voix : {voices:?}"
        );
        let id = engine
            .speak(
                "Bonjour.",
                &VoiceSettings {
                    volume: 0.0,
                    ..settings()
                },
            )
            .unwrap();
        let ended = rx.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        assert_eq!(ended, id);
    }

    /// Le moteur indisponible refuse de lire, avec sa raison.
    #[test]
    fn unavailable_engine_explains_itself() {
        let engine = UnavailableEngine("aucune voix".into());
        assert_eq!(
            engine.speak("x", &settings()).unwrap_err().to_string(),
            "synthèse vocale : aucune voix"
        );
        assert!(engine.voices().unwrap().is_empty());
        engine.stop().unwrap();
    }
}
