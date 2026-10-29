//! Commandes du mode Dictée (F-10 à F-17, ADR 0020) : voix, réglages et profils,
//! session de dictée (lecture des segments, brouillon, correction, reprise).
//!
//! Le texte reste côté backend jusqu'à la correction : l'interface ne connaît que
//! le nombre de segments et de mots (F-10).

use dactylo_core::dictation::advance::typed_words;
use dactylo_core::dictation::punctuation::spoken;
use dactylo_core::dictation::segment::segments;
use dactylo_core::dictation::settings::DictationSettings;
use dactylo_core::dictation::{Correction, correct, session_words};
use dactylo_core::events::DomainEvent;
use dactylo_core::exercise::{exercises, next_exercise};
use dactylo_core::speech::{Voice, VoiceSettings};
use dactylo_core::text::graphemes;
use serde::Serialize;
use tauri::State;

use crate::db::dictation::{self as repo, DictationNumbers};
use crate::db::{profiles, sessions, texts};
use crate::error::{AppError, AppResult};
use crate::state::{AppState, LiveDictation, SessionExercise};

/// Clé des réglages du module dans `profiles.settings` (ADR 0010).
const SETTINGS_KEY: &str = "dictation";
/// Dictées de la moyenne affichée à la correction.
const AVERAGE_OVER: u32 = 10;

/// Rang de l'exercice dicté, pour « exercice 3 sur 8 ».
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DictationExercise {
    /// Rang, à partir de 0.
    pub index: u32,
    /// Nombre d'exercices du texte.
    pub count: u32,
}

impl From<SessionExercise> for DictationExercise {
    /// Rang et nombre, sans la portion du texte.
    fn from(e: SessionExercise) -> Self {
        Self {
            index: e.index,
            count: e.count,
        }
    }
}

/// Ce qu'il faut à l'interface pour mener une dictée, sans le texte (F-10).
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DictationStart {
    /// Identifiant de la session.
    pub session_id: String,
    /// Texte dicté.
    pub text_id: String,
    /// Titre du texte.
    pub title: String,
    /// Exercice dicté.
    pub exercise: DictationExercise,
    /// Mots de chaque segment, dans l'ordre de lecture.
    pub segment_words: Vec<u32>,
    /// Mots du texte dicté.
    pub total_words: u32,
    /// Réglages de la dictée.
    pub settings: DictationSettings,
    /// Texte déjà tapé (reprise) ; vide pour une nouvelle dictée.
    pub typed: String,
    /// Temps actif déjà écoulé, en ms.
    pub active_ms: u32,
    /// Répétitions déjà faites de chaque segment.
    pub repeats: Vec<u32>,
}

/// Dictée interrompue proposée à la reprise (NF-06).
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PendingDictation {
    /// Identifiant de la session.
    pub session_id: String,
    /// Texte dicté.
    pub text_id: String,
    /// Titre du texte.
    pub title: String,
    /// Exercice interrompu.
    pub exercise: DictationExercise,
    /// Mots déjà tapés.
    pub typed_words: u32,
    /// Mots du texte dicté.
    pub total_words: u32,
    /// Début, en ms Unix.
    pub started_at: i64,
}

/// Segment dont la lecture vient de commencer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SpokenSegment {
    /// Numéro de lecture, repris par l'événement `SpeechFinished`.
    pub utterance: u64,
    /// Rang du segment.
    pub index: u32,
    /// Répétitions faites de ce segment.
    pub repeats: u32,
}

/// Chiffres d'une dictée corrigée.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DictationStats {
    /// Temps actif, en ms.
    pub duration_ms: u32,
    /// Mots tapés par minute.
    pub wpm: f64,
    /// Précision en mots, en %.
    pub accuracy: f64,
    /// Fautes, ponctuation comprise.
    pub faults: u32,
    /// Répétitions demandées, tous segments confondus.
    pub repeats: u32,
}

/// Moyenne des dictées précédentes, pour la comparaison.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DictationAverage {
    /// Dictées prises en compte.
    pub sessions: u32,
    /// Précision moyenne, en %.
    pub accuracy: f64,
    /// Fautes moyennes.
    pub faults: f64,
    /// MPM moyen.
    pub wpm: f64,
}

/// Correction d'une dictée (F-16) : le texte est enfin révélé.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DictationSummary {
    /// Identifiant de la session.
    pub session_id: String,
    /// Texte dicté.
    pub text_id: String,
    /// Titre du texte.
    pub title: String,
    /// Exercice dicté.
    pub exercise: DictationExercise,
    /// Exercice suivant du même texte, s'il y en a un.
    pub next_exercise: Option<u32>,
    /// Alignement et fautes.
    pub correction: Correction,
    /// Chiffres.
    pub stats: DictationStats,
    /// Moyenne des dictées précédentes ; `None` pour la première.
    pub average: Option<DictationAverage>,
}

/// Profil de dictée enregistré (F-17).
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DictationProfile {
    /// Identifiant.
    pub id: String,
    /// Nom.
    pub name: String,
    /// Réglages.
    pub settings: DictationSettings,
}

/// Réglages de dictée du profil actif ; défauts si absents.
pub async fn load_settings(state: &AppState) -> AppResult<DictationSettings> {
    let value = profiles::get_settings(&state.pool, &state.profile_id, SETTINGS_KEY).await?;
    Ok(DictationSettings::from_json(value.as_ref()))
}

/// Borne puis enregistre les réglages ; renvoie la version stockée.
pub async fn store_settings(
    state: &AppState,
    settings: DictationSettings,
) -> AppResult<DictationSettings> {
    let settings = settings.sanitized();
    let value = serde_json::to_value(&settings).map_err(|e| AppError::Invalid(e.to_string()))?;
    profiles::save_settings(&state.pool, &state.profile_id, SETTINGS_KEY, &value).await?;
    Ok(settings)
}

/// Réglages et exercice rangés dans `sessions.settings`, pour la reprise.
fn session_json(settings: &DictationSettings, exercise: SessionExercise) -> AppResult<String> {
    let invalid = |e: serde_json::Error| AppError::Invalid(e.to_string());
    Ok(serde_json::json!({
        "dictation": serde_json::to_value(settings).map_err(invalid)?,
        "exercise": serde_json::to_value(exercise).map_err(invalid)?,
    })
    .to_string())
}

/// Dictée en mémoire à partir d'un texte, d'un exercice et de réglages.
fn live(
    profile_id: &str,
    text: &texts::TextContent,
    exercise: SessionExercise,
    settings: DictationSettings,
) -> AppResult<LiveDictation> {
    // Portion du texte de l'exercice ; bornes ramenées dans le texte s'il a changé.
    let all = graphemes(&text.content);
    let end = (exercise.end as usize).min(all.len());
    let start = (exercise.start as usize).min(end);
    let content = all[start..end].concat();
    let parts = segments(
        &graphemes(&content),
        settings.segmentation,
        settings.group_size,
    );
    if parts.is_empty() {
        return Err(AppError::Invalid("aucun mot à dicter dans ce texte".into()));
    }
    Ok(LiveDictation {
        profile_id: profile_id.to_owned(),
        text_id: text.id.clone(),
        title: text.title.clone(),
        exercise,
        repeats: vec![0; parts.len()],
        settings,
        content,
        segments: parts,
    })
}

/// Données de démarrage envoyées à l'interface (sans le texte).
fn start_info(
    session_id: &str,
    d: &LiveDictation,
    typed: String,
    active_ms: u32,
) -> DictationStart {
    let segment_words: Vec<u32> = d.segments.iter().map(|s| s.words).collect();
    DictationStart {
        session_id: session_id.to_owned(),
        text_id: d.text_id.clone(),
        title: d.title.clone(),
        exercise: d.exercise.into(),
        total_words: segment_words.iter().sum(),
        segment_words,
        settings: d.settings.clone(),
        typed,
        active_ms,
        repeats: d.repeats.clone(),
    }
}

/// Démarre une dictée sur `text_id` (ou le texte le plus récent), sur l'exercice
/// demandé ou le prochain du profil (F-24), avec les réglages du profil.
pub async fn start(
    state: &AppState,
    text_id: Option<String>,
    exercise: Option<u32>,
) -> AppResult<DictationStart> {
    // Sans texte précisé, le plus récent de la bibliothèque.
    let text_id = match text_id {
        Some(id) => id,
        None => texts::latest(&state.pool)
            .await?
            .ok_or_else(|| AppError::NotFound("aucun texte dans la bibliothèque".into()))?,
    };
    let text = texts::get(&state.pool, &text_id).await?;
    let settings = load_settings(state).await?;
    // Exercice demandé s'il existe, sinon le prochain à faire.
    let spans = exercises(&graphemes(&text.content), text.segmentation);
    let count = spans.len() as u32;
    let index = match exercise {
        Some(i) if i < count => i,
        Some(i) => {
            return Err(AppError::Invalid(format!(
                "exercice {} absent : le texte en compte {count}",
                i + 1
            )));
        }
        None => {
            let done = texts::progress(&state.pool, &state.profile_id, &text.id).await?;
            next_exercise(done, spans.len())
        }
    };
    let span = spans[index as usize];
    let exercise = SessionExercise {
        index,
        count,
        start: span.start,
        end: span.end,
    };
    let dictation = live(&state.profile_id, &text, exercise, settings)?;
    // Session enregistrée ; une autre session en cours du profil est abandonnée.
    let row = sessions::start(
        &state.pool,
        &state.profile_id,
        Some(&text.id),
        sessions::MODE_DICTATION,
        &session_json(&dictation.settings, exercise)?,
    )
    .await?;
    texts::touch_progress(&state.pool, &state.profile_id, &text.id, None).await?;
    let info = start_info(&row.id, &dictation, String::new(), 0);
    state.put_dictation(row.id, dictation);
    Ok(info)
}

/// Dictée en cours, depuis le cache ou reconstruite depuis la base. Elle est retirée
/// du cache : l'appelant la remet s'il la garde.
async fn take(state: &AppState, session_id: &str) -> AppResult<LiveDictation> {
    if let Some(d) = state.take_dictation(session_id) {
        return Ok(d);
    }
    // Reconstruction (redémarrage) : texte, exercice et réglages de la session.
    let row = sessions::get(&state.pool, session_id).await?;
    if row.mode != sessions::MODE_DICTATION || row.ended_at.is_some() || row.abandoned_at.is_some()
    {
        return Err(AppError::Invalid(format!(
            "dictée {session_id} terminée, abandonnée ou absente"
        )));
    }
    let text_id = row
        .text_id
        .ok_or_else(|| AppError::NotFound(format!("texte de la dictée {session_id}")))?;
    let text = texts::get(&state.pool, &text_id).await?;
    let value: Option<serde_json::Value> = serde_json::from_str(&row.settings).ok();
    let settings = DictationSettings::from_json(value.as_ref().and_then(|v| v.get("dictation")));
    let exercise = value
        .as_ref()
        .and_then(|v| v.get("exercise"))
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or(SessionExercise {
            index: 0,
            count: 1,
            start: 0,
            end: graphemes(&text.content).len() as u32,
        });
    live(&row.profile_id, &text, exercise, settings)
}

/// Lit le segment `index` (F-10) ; `repeat` compte une répétition (F-14), refusée
/// au-delà de la limite réglée.
pub async fn speak_segment(
    state: &AppState,
    session_id: &str,
    index: u32,
    repeat: bool,
) -> AppResult<SpokenSegment> {
    let mut d = take(state, session_id).await?;
    let i = index as usize;
    // Le résultat est calculé avant de remettre la dictée dans le cache.
    let result = (|| {
        let segment = d
            .segments
            .get(i)
            .ok_or_else(|| AppError::Invalid(format!("segment {} absent", index + 1)))?;
        // Répétition : seulement dans la limite des réglages.
        if repeat {
            if !d.settings.can_repeat(d.repeats[i]) {
                return Err(AppError::Invalid(
                    "plus de répétition possible pour ce segment".into(),
                ));
            }
            d.repeats[i] += 1;
        }
        let voice = VoiceSettings {
            voice: d.settings.voice.clone(),
            rate: d.settings.rate,
            pitch: d.settings.pitch,
            volume: d.settings.volume,
        };
        let utterance = state
            .speech
            .speak(&spoken(&segment.text, d.settings.read_punctuation), &voice)?;
        Ok(SpokenSegment {
            utterance,
            index,
            repeats: d.repeats[i],
        })
    })();
    state.put_dictation(session_id.to_owned(), d);
    result
}

/// Change les réglages d'une dictée en cours : voix, vitesse, pause, avancement…
/// s'appliquent au segment suivant. Le découpage reste celui du début (les segments
/// sont figés) ; le changer demande une nouvelle dictée.
pub async fn update_settings(
    state: &AppState,
    session_id: &str,
    settings: DictationSettings,
) -> AppResult<DictationSettings> {
    let mut d = take(state, session_id).await?;
    d.settings = DictationSettings {
        segmentation: d.settings.segmentation,
        group_size: d.settings.group_size,
        ..settings.sanitized()
    };
    let json = session_json(&d.settings, d.exercise);
    let effective = d.settings.clone();
    state.put_dictation(session_id.to_owned(), d);
    repo::update_session_settings(&state.pool, session_id, &json?).await?;
    Ok(effective)
}

/// Enregistre le brouillon (texte tapé, temps actif) pour la reprise (NF-06).
pub async fn save_draft(
    state: &AppState,
    session_id: &str,
    typed: &str,
    active_ms: u32,
) -> AppResult<()> {
    repo::save_draft(&state.pool, session_id, typed, active_ms).await
}

/// Corrige la dictée (F-16), l'enregistre et publie ses mots pour la révision.
pub async fn finish(
    state: &AppState,
    session_id: &str,
    typed: &str,
    active_ms: u32,
) -> AppResult<DictationSummary> {
    let d = take(state, session_id).await?;
    let _ = state.speech.stop();
    let correction = correct(&d.content, typed, d.settings.lenient_punctuation);
    // Vitesse en mots et en caractères tapés par minute ; précision en mots attendus.
    let minutes = f64::from(active_ms) / 60_000.0;
    let per_minute = |n: f64| if minutes > 0.0 { n / minutes } else { 0.0 };
    let accuracy = if correction.expected_words == 0 {
        100.0
    } else {
        100.0 * f64::from(correction.correct_words) / f64::from(correction.expected_words)
    };
    let numbers = DictationNumbers {
        duration_ms: active_ms,
        wpm: per_minute(f64::from(typed_words(typed))),
        cpm: per_minute(graphemes(typed.trim()).len() as f64),
        accuracy,
    };
    repo::finish(&state.pool, session_id, typed, numbers, &correction).await?;
    let average = repo::average(&state.pool, &d.profile_id, session_id, AVERAGE_OVER).await?;

    // Événements après l'écriture : les mots vont à la révision (ADR 0015, 0021).
    state.bus.publish(&DomainEvent::SessionCompleted {
        session_id: session_id.to_owned(),
        profile_id: d.profile_id.clone(),
        words: session_words(&correction),
    });
    for t in correction
        .tokens
        .iter()
        .filter(|t| t.word && t.error.is_some())
    {
        if let (Some(expected), typed) = (&t.expected, &t.typed) {
            state.bus.publish(&DomainEvent::WordMistyped {
                session_id: session_id.to_owned(),
                profile_id: d.profile_id.clone(),
                expected: expected.clone(),
                typed: typed.clone().unwrap_or_default(),
            });
        }
    }
    let exercise = d.exercise;
    Ok(DictationSummary {
        session_id: session_id.to_owned(),
        text_id: d.text_id,
        title: d.title,
        exercise: exercise.into(),
        next_exercise: (exercise.index + 1 < exercise.count).then_some(exercise.index + 1),
        stats: DictationStats {
            duration_ms: active_ms,
            wpm: numbers.wpm,
            accuracy,
            faults: correction.faults,
            repeats: d.repeats.iter().sum(),
        },
        average: average.map(|a| DictationAverage {
            sessions: a.sessions,
            accuracy: a.accuracy,
            faults: a.faults,
            wpm: a.wpm,
        }),
        correction,
    })
}

/// Abandonne une dictée : le brouillon `typed` est d'abord enregistré, puis la
/// session est gardée comme abandonnée (ou supprimée si rien n'a été tapé, ADR 0014).
pub async fn abandon(
    state: &AppState,
    session_id: &str,
    typed: &str,
    active_ms: u32,
) -> AppResult<()> {
    state.take_dictation(session_id);
    let _ = state.speech.stop();
    // Une dictée déjà close n'a plus de brouillon à garder.
    if !typed.trim().is_empty() {
        let _ = repo::save_draft(&state.pool, session_id, typed, active_ms).await;
    }
    sessions::abandon(&state.pool, session_id).await
}

/// Dictée interrompue à proposer à la reprise, s'il y en a une (NF-06).
pub async fn pending(state: &AppState) -> AppResult<Option<PendingDictation>> {
    let Some(row) =
        sessions::pending(&state.pool, &state.profile_id, sessions::Family::Dictation).await?
    else {
        return Ok(None);
    };
    let d = take(state, &row.id).await?;
    let (typed, _) = repo::draft(&state.pool, &row.id).await?;
    let pending = PendingDictation {
        session_id: row.id.clone(),
        text_id: d.text_id.clone(),
        title: d.title.clone(),
        exercise: d.exercise.into(),
        typed_words: typed_words(&typed),
        total_words: d.segments.iter().map(|s| s.words).sum(),
        started_at: row.started_at,
    };
    state.put_dictation(row.id, d);
    Ok(Some(pending))
}

/// Reprend une dictée interrompue, avec son brouillon.
pub async fn resume(state: &AppState, session_id: &str) -> AppResult<DictationStart> {
    let d = take(state, session_id).await?;
    let (typed, active_ms) = repo::draft(&state.pool, session_id).await?;
    let info = start_info(session_id, &d, typed, active_ms);
    state.put_dictation(session_id.to_owned(), d);
    Ok(info)
}

/// Profil de dictée tel que vu par l'interface.
fn to_profile(r: repo::ProfileRecord) -> DictationProfile {
    DictationProfile {
        id: r.id,
        name: r.name,
        settings: r.settings,
    }
}

/// Commande : voix installées (F-12).
#[tauri::command]
#[specta::specta]
pub async fn list_voices(state: State<'_, AppState>) -> AppResult<Vec<Voice>> {
    Ok(state.speech.voices()?)
}

/// Commande : réglages de dictée du profil.
#[tauri::command]
#[specta::specta]
pub async fn get_dictation_settings(state: State<'_, AppState>) -> AppResult<DictationSettings> {
    load_settings(&state).await
}

/// Commande : enregistre les réglages de dictée du profil.
#[tauri::command]
#[specta::specta]
pub async fn save_dictation_settings(
    state: State<'_, AppState>,
    settings: DictationSettings,
) -> AppResult<DictationSettings> {
    store_settings(&state, settings).await
}

/// Commande : profils de dictée enregistrés (F-17).
#[tauri::command]
#[specta::specta]
pub async fn list_dictation_profiles(
    state: State<'_, AppState>,
) -> AppResult<Vec<DictationProfile>> {
    let list = repo::list_profiles(&state.pool, &state.profile_id).await?;
    Ok(list.into_iter().map(to_profile).collect())
}

/// Commande : crée (sans `id`) ou remplace un profil de dictée (F-17).
#[tauri::command]
#[specta::specta]
pub async fn save_dictation_profile(
    state: State<'_, AppState>,
    id: Option<String>,
    name: String,
    settings: DictationSettings,
) -> AppResult<DictationProfile> {
    let id = repo::save_profile(
        &state.pool,
        &state.profile_id,
        id.as_deref(),
        &name,
        &settings,
    )
    .await?;
    Ok(DictationProfile {
        id,
        name: name.trim().to_owned(),
        settings: settings.sanitized(),
    })
}

/// Commande : supprime un profil de dictée (F-17).
#[tauri::command]
#[specta::specta]
pub async fn delete_dictation_profile(state: State<'_, AppState>, id: String) -> AppResult<()> {
    repo::delete_profile(&state.pool, &state.profile_id, &id).await
}

/// Commande : démarre une dictée.
#[tauri::command]
#[specta::specta]
pub async fn start_dictation(
    state: State<'_, AppState>,
    text_id: Option<String>,
    exercise: Option<u32>,
) -> AppResult<DictationStart> {
    start(&state, text_id, exercise).await
}

/// Commande : lit un segment, en répétition ou non (F-10, F-14).
#[tauri::command]
#[specta::specta]
pub async fn speak_dictation_segment(
    state: State<'_, AppState>,
    session_id: String,
    index: u32,
    repeat: bool,
) -> AppResult<SpokenSegment> {
    speak_segment(&state, &session_id, index, repeat).await
}

/// Commande : interrompt la lecture (pause).
#[tauri::command]
#[specta::specta]
pub async fn stop_speech(state: State<'_, AppState>) -> AppResult<()> {
    Ok(state.speech.stop()?)
}

/// Commande : change les réglages d'une dictée en cours.
#[tauri::command]
#[specta::specta]
pub async fn update_dictation_settings(
    state: State<'_, AppState>,
    session_id: String,
    settings: DictationSettings,
) -> AppResult<DictationSettings> {
    update_settings(&state, &session_id, settings).await
}

/// Commande : enregistre le brouillon (NF-06).
#[tauri::command]
#[specta::specta]
pub async fn save_dictation_draft(
    state: State<'_, AppState>,
    session_id: String,
    typed: String,
    active_ms: u32,
) -> AppResult<()> {
    save_draft(&state, &session_id, &typed, active_ms).await
}

/// Commande : corrige la dictée (F-16).
#[tauri::command]
#[specta::specta]
pub async fn finish_dictation(
    state: State<'_, AppState>,
    session_id: String,
    typed: String,
    active_ms: u32,
) -> AppResult<DictationSummary> {
    finish(&state, &session_id, &typed, active_ms).await
}

/// Commande : abandonne la dictée, brouillon gardé.
#[tauri::command]
#[specta::specta]
pub async fn abandon_dictation(
    state: State<'_, AppState>,
    session_id: String,
    typed: String,
    active_ms: u32,
) -> AppResult<()> {
    abandon(&state, &session_id, &typed, active_ms).await
}

/// Commande : dictée à reprendre, s'il y en a une.
#[tauri::command]
#[specta::specta]
pub async fn pending_dictation(state: State<'_, AppState>) -> AppResult<Option<PendingDictation>> {
    pending(&state).await
}

/// Commande : reprend une dictée interrompue.
#[tauri::command]
#[specta::specta]
pub async fn resume_dictation(
    state: State<'_, AppState>,
    session_id: String,
) -> AppResult<DictationStart> {
    resume(&state, &session_id).await
}

/// Tests des commandes de dictée (dans `dictation/tests.rs`).
#[cfg(test)]
mod tests;
