//! Tableau de bord (F-33) : courbe de progression, temps total, record,
//! série de jours consécutifs, tendance et semaine en cours (ADR 0017).
//!
//! Tout est calculé à partir des sessions du profil, terminées ou abandonnées.
//! Seules les terminées ont des statistiques ; les abandonnées comptent dans le
//! temps d'entraînement et la série de jours.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::days::{day_start, local_day, week_start};

/// Nombre de sessions de chaque fenêtre de la tendance (« +3 sur 10 sessions »).
pub const TREND_WINDOW: usize = 10;

/// Période affichée par le tableau de bord.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum Period {
    /// Les 7 derniers jours, aujourd'hui compris.
    Week,
    /// Les 30 derniers jours, aujourd'hui compris.
    #[default]
    Month,
    /// Depuis le début.
    All,
}

impl Period {
    /// Début de la période (minuit local), ou `None` pour « tout ».
    pub fn start(self, now_ms: i64, tz_offset_min: i32) -> Option<i64> {
        // Nombre de jours couverts, aujourd'hui compris.
        let days = match self {
            Period::Week => 7,
            Period::Month => 30,
            Period::All => return None,
        };
        let today = local_day(now_ms, tz_offset_min);
        Some(day_start(today - (days - 1), tz_offset_min))
    }
}

/// Une session du profil, telle que lue en base.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SessionSample {
    /// Début, en ms Unix.
    pub started_at: i64,
    /// Temps actif, en ms.
    pub duration_ms: u32,
    /// Statistiques, pour une session terminée ; `None` si abandonnée.
    pub result: Option<SessionResult>,
}

/// Statistiques retenues d'une session terminée.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SessionResult {
    /// MPM net.
    pub wpm_net: f64,
    /// Précision, en %.
    pub accuracy: f64,
}

/// Point de la courbe de progression.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ProgressPoint {
    /// Début de la session, en ms Unix.
    pub started_at: i64,
    /// MPM net.
    pub wpm_net: f64,
    /// Précision, en %.
    pub accuracy: f64,
}

/// Série de jours consécutifs d'entraînement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Streak {
    /// Série en cours : jusqu'à aujourd'hui, ou jusqu'à hier si rien encore aujourd'hui.
    pub current: u32,
    /// Plus longue série.
    pub best: u32,
}

/// Moyenne des dernières sessions et écart avec les précédentes.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Trend {
    /// MPM net moyen des `TREND_WINDOW` dernières sessions terminées.
    pub wpm_net: Option<f64>,
    /// Écart avec les `TREND_WINDOW` sessions d'avant, s'il y en a.
    pub wpm_net_delta: Option<f64>,
    /// Précision moyenne des dernières sessions, en %.
    pub accuracy: Option<f64>,
    /// Écart de précision, en points.
    pub accuracy_delta: Option<f64>,
}

/// Temps et nombre de sessions sur une période.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    /// Temps actif total, en ms.
    pub training_ms: u64,
    /// Sessions terminées.
    pub sessions: u32,
}

/// Tableau de bord d'un profil.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Dashboard {
    /// Sessions terminées de la période, dans l'ordre chronologique.
    pub points: Vec<ProgressPoint>,
    /// Temps et sessions de la période.
    pub period: Activity,
    /// Précision moyenne des sessions terminées de la période, en %.
    pub average_accuracy: Option<f64>,
    /// Meilleur MPM net de tous les temps (record personnel).
    pub best_wpm_net: Option<f64>,
    /// Série de jours consécutifs.
    pub streak: Streak,
    /// Tendance des dernières sessions, toutes périodes confondues.
    pub trend: Trend,
    /// Semaine locale en cours (depuis lundi).
    pub week: Activity,
}

/// Calcule le tableau de bord. `sessions` : toutes les sessions du profil,
/// dans n'importe quel ordre ; `now_ms` et `tz_offset_min` fixent « aujourd'hui ».
pub fn dashboard(
    sessions: &[SessionSample],
    period: Period,
    now_ms: i64,
    tz_offset_min: i32,
) -> Dashboard {
    // Ordre chronologique : la courbe et la tendance en dépendent.
    let mut sorted = sessions.to_vec();
    sorted.sort_by_key(|s| s.started_at);
    let from = period.start(now_ms, tz_offset_min).unwrap_or(i64::MIN);
    let in_period: Vec<&SessionSample> = sorted.iter().filter(|s| s.started_at >= from).collect();

    // Courbe : les sessions terminées de la période.
    let points: Vec<ProgressPoint> = in_period
        .iter()
        .filter_map(|s| {
            s.result.map(|r| ProgressPoint {
                started_at: s.started_at,
                wpm_net: r.wpm_net,
                accuracy: r.accuracy,
            })
        })
        .collect();
    let average_accuracy = mean(points.iter().map(|p| p.accuracy));

    // Record de tous les temps, parmi les sessions terminées.
    let best_wpm_net = sorted
        .iter()
        .filter_map(|s| s.result.map(|r| r.wpm_net))
        .reduce(f64::max);

    // Semaine en cours, depuis lundi minuit local.
    let monday = week_start(local_day(now_ms, tz_offset_min));
    let week_from = day_start(monday, tz_offset_min);
    let week: Vec<&SessionSample> = sorted
        .iter()
        .filter(|s| s.started_at >= week_from)
        .collect();

    Dashboard {
        period: activity(&in_period),
        average_accuracy,
        best_wpm_net,
        streak: streak(&sorted, now_ms, tz_offset_min),
        trend: trend(&sorted),
        week: activity(&week),
        points,
    }
}

/// Temps total et sessions terminées d'une liste de sessions.
fn activity(sessions: &[&SessionSample]) -> Activity {
    Activity {
        training_ms: sessions.iter().map(|s| u64::from(s.duration_ms)).sum(),
        sessions: sessions.iter().filter(|s| s.result.is_some()).count() as u32,
    }
}

/// Moyenne arithmétique ; `None` sans valeur.
fn mean(values: impl Iterator<Item = f64>) -> Option<f64> {
    let (sum, n) = values.fold((0.0, 0u32), |(sum, n), v| (sum + v, n + 1));
    (n > 0).then(|| sum / f64::from(n))
}

/// Série de jours locaux consécutifs avec au moins une session.
fn streak(sessions: &[SessionSample], now_ms: i64, tz_offset_min: i32) -> Streak {
    let days: BTreeSet<i64> = sessions
        .iter()
        .map(|s| local_day(s.started_at, tz_offset_min))
        .collect();
    // Plus longue suite de jours qui se suivent.
    let mut best = 0;
    let mut run = 0;
    let mut previous: Option<i64> = None;
    for &day in &days {
        run = if previous == Some(day - 1) {
            run + 1
        } else {
            1
        };
        best = best.max(run);
        previous = Some(day);
    }
    // Série en cours : rien aujourd'hui ne la casse encore, elle peut finir hier.
    let today = local_day(now_ms, tz_offset_min);
    let mut day = if days.contains(&today) {
        today
    } else {
        today - 1
    };
    let mut current = 0;
    while days.contains(&day) {
        current += 1;
        day -= 1;
    }
    Streak { current, best }
}

/// Moyenne des dernières sessions terminées et écart avec la fenêtre précédente.
fn trend(sorted: &[SessionSample]) -> Trend {
    // Les plus récentes d'abord.
    let results: Vec<SessionResult> = sorted.iter().rev().filter_map(|s| s.result).collect();
    let window = |skip: usize| -> Vec<SessionResult> {
        results
            .iter()
            .skip(skip)
            .take(TREND_WINDOW)
            .copied()
            .collect()
    };
    let (last, before) = (window(0), window(TREND_WINDOW));
    // Moyennes d'un champ sur une fenêtre.
    let wpm = |w: &[SessionResult]| mean(w.iter().map(|r| r.wpm_net));
    let acc = |w: &[SessionResult]| mean(w.iter().map(|r| r.accuracy));
    // Un écart n'a de sens que si les deux fenêtres ont des sessions.
    let delta = |a: Option<f64>, b: Option<f64>| a.zip(b).map(|(a, b)| a - b);
    Trend {
        wpm_net: wpm(&last),
        wpm_net_delta: delta(wpm(&last), wpm(&before)),
        accuracy: acc(&last),
        accuracy_delta: delta(acc(&last), acc(&before)),
    }
}

/// Tests du tableau de bord.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::days::DAY_MS;

    /// Jeudi 29 octobre 2026, 12 h UTC.
    const NOW: i64 = 1_793_232_000_000 + 12 * 3_600_000;

    /// Session terminée `days_ago` jours avant `NOW`.
    fn done(days_ago: i64, wpm_net: f64, accuracy: f64) -> SessionSample {
        SessionSample {
            started_at: NOW - days_ago * DAY_MS,
            duration_ms: 60_000,
            result: Some(SessionResult { wpm_net, accuracy }),
        }
    }

    /// Session abandonnée `days_ago` jours avant `NOW`.
    fn abandoned(days_ago: i64) -> SessionSample {
        SessionSample {
            started_at: NOW - days_ago * DAY_MS,
            duration_ms: 30_000,
            result: None,
        }
    }

    /// Aucune session : tableau vide, sans moyenne ni record.
    #[test]
    fn empty_dashboard() {
        let d = dashboard(&[], Period::Month, NOW, 0);
        assert_eq!(d, Dashboard::default());
    }

    /// La période filtre la courbe et le temps ; le record reste de tous les temps.
    #[test]
    fn period_filters_points_but_not_record() {
        let sessions = [
            done(40, 70.0, 90.0),
            done(10, 40.0, 95.0),
            done(1, 50.0, 97.0),
        ];
        let d = dashboard(&sessions, Period::Month, NOW, 0);
        let wpm: Vec<f64> = d.points.iter().map(|p| p.wpm_net).collect();
        assert_eq!(wpm, [40.0, 50.0]);
        assert_eq!(d.period.sessions, 2);
        assert_eq!(d.period.training_ms, 120_000);
        assert_eq!(d.average_accuracy, Some(96.0));
        assert_eq!(d.best_wpm_net, Some(70.0));
        // « Tout » reprend la session d'il y a 40 jours.
        assert_eq!(dashboard(&sessions, Period::All, NOW, 0).points.len(), 3);
        // 7 jours : seule celle d'hier.
        assert_eq!(dashboard(&sessions, Period::Week, NOW, 0).points.len(), 1);
    }

    /// Les sessions abandonnées comptent dans le temps, pas dans la courbe.
    #[test]
    fn abandoned_sessions_count_time_only() {
        let d = dashboard(&[abandoned(0), done(0, 45.0, 96.0)], Period::Week, NOW, 0);
        assert_eq!(d.points.len(), 1);
        assert_eq!(d.period.training_ms, 90_000);
        assert_eq!(d.period.sessions, 1);
    }

    /// La courbe est chronologique, quel que soit l'ordre reçu.
    #[test]
    fn points_are_chronological() {
        let d = dashboard(
            &[done(1, 50.0, 97.0), done(3, 40.0, 95.0)],
            Period::Week,
            NOW,
            0,
        );
        assert!(d.points[0].started_at < d.points[1].started_at);
    }

    /// Série en cours et record, avec un trou.
    #[test]
    fn streak_current_and_best() {
        // Jours -9 à -6 (4 jours), trou, puis -2 à 0 (3 jours).
        let sessions: Vec<_> = [9, 8, 7, 6, 2, 1, 0].map(abandoned).into();
        let d = dashboard(&sessions, Period::All, NOW, 0);
        assert_eq!(
            d.streak,
            Streak {
                current: 3,
                best: 4
            }
        );
    }

    /// Rien aujourd'hui : la série d'hier n'est pas encore cassée. Avant-hier, si.
    #[test]
    fn streak_survives_until_end_of_today() {
        let yesterday = [abandoned(2), abandoned(1)];
        assert_eq!(dashboard(&yesterday, Period::All, NOW, 0).streak.current, 2);
        let older = [abandoned(3), abandoned(2)];
        assert_eq!(dashboard(&older, Period::All, NOW, 0).streak.current, 0);
        assert_eq!(dashboard(&older, Period::All, NOW, 0).streak.best, 2);
    }

    /// Le fuseau horaire décide du jour d'une session tardive.
    #[test]
    fn streak_uses_local_days() {
        // Hier 23 h 30 UTC = aujourd'hui 0 h 30 à Paris (+60) ; plus une session avant-hier.
        let late = SessionSample {
            started_at: NOW - 12 * 3_600_000 - 30 * 60_000,
            duration_ms: 1,
            result: None,
        };
        let sessions = [late, abandoned(2)];
        // En UTC : hier et avant-hier se suivent.
        assert_eq!(dashboard(&sessions, Period::All, NOW, 0).streak.current, 2);
        // À Paris : aujourd'hui et avant-hier, la série est coupée.
        let paris = dashboard(&sessions, Period::All, NOW, 60).streak;
        assert_eq!(
            paris,
            Streak {
                current: 1,
                best: 1
            }
        );
    }

    /// Tendance : moyenne des 10 dernières et écart avec les 10 d'avant.
    #[test]
    fn trend_compares_two_windows() {
        // 10 anciennes à 40 MPM / 95 %, puis 10 récentes à 43 MPM / 96 %.
        let mut sessions: Vec<_> = (0..10).map(|i| done(30 - i, 40.0, 95.0)).collect();
        sessions.extend((0..10).map(|i| done(10 - i, 43.0, 96.0)));
        let t = dashboard(&sessions, Period::All, NOW, 0).trend;
        assert_eq!(t.wpm_net, Some(43.0));
        assert_eq!(t.wpm_net_delta, Some(3.0));
        assert_eq!(t.accuracy, Some(96.0));
        assert_eq!(t.accuracy_delta, Some(1.0));
    }

    /// Moins de 11 sessions : moyenne, mais pas d'écart.
    #[test]
    fn trend_without_previous_window_has_no_delta() {
        let t = dashboard(&[done(1, 50.0, 97.0)], Period::All, NOW, 0).trend;
        assert_eq!(t.wpm_net, Some(50.0));
        assert_eq!(t.wpm_net_delta, None);
    }

    /// La semaine part du lundi : jeudi, une session de lundi compte, pas celle de dimanche.
    #[test]
    fn week_starts_on_monday() {
        let sessions = [done(3, 40.0, 95.0), done(4, 40.0, 95.0), abandoned(0)];
        let w = dashboard(&sessions, Period::All, NOW, 0).week;
        assert_eq!(w.sessions, 1);
        assert_eq!(w.training_ms, 90_000);
    }

    /// Débuts de période, minuit local compris.
    #[test]
    fn period_starts() {
        let today = day_start(local_day(NOW, 0), 0);
        assert_eq!(Period::Week.start(NOW, 0), Some(today - 6 * DAY_MS));
        assert_eq!(Period::Month.start(NOW, 0), Some(today - 29 * DAY_MS));
        assert_eq!(Period::All.start(NOW, 0), None);
    }
}
