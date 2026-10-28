/** État de la session de copie en cours (Zustand). */

import { create } from 'zustand';

import type { Keystroke_Deserialize as Keystroke, SessionStart_Serialize } from '@/shared/api';

import { isGoalReached, TypingEngine, type RawKey } from './engine/session';

/** Données de démarrage d'une session, telles que renvoyées par le backend. */
export type SessionStart = SessionStart_Serialize;

/** Étape de la session : frappe, pause, envoi de la fin, résumé affiché. */
export type Status = 'typing' | 'paused' | 'finishing' | 'done';

/**
 * Chronomètre du temps actif : il démarre à la première frappe, s'arrête en pause
 * et repart à la frappe suivante (ADR 0009).
 */
interface Clock {
  /** Temps actif déjà cumulé, en ms. */
  accumulated: number;
  /** Instant de reprise du chronomètre ; `null` s'il est arrêté. */
  runningSince: number | null;
}

/** État et actions du store d'entraînement. */
interface PracticeState {
  /** Session chargée. */
  session: SessionStart | null;
  /** Miroir de comparaison de frappe. */
  engine: TypingEngine | null;
  /** Incrémenté à chaque frappe acceptée : l'affichage s'y abonne. */
  version: number;
  /** Étape de la session. */
  status: Status;
  /** Chronomètre du temps actif. */
  clock: Clock;
  /** Numéro de la prochaine frappe. */
  nextSeq: number;
  /** Frappes pas encore envoyées au backend. */
  queue: Keystroke[];

  /** Charge une session (nouvelle ou reprise). */
  load: (start: SessionStart) => void;
  /** Traite une frappe à l'instant `now`. */
  input: (key: RawKey, now: number) => void;
  /** Met en pause ou reprend. */
  togglePause: (now: number) => void;
  /** Passe en fin de session si l'objectif est atteint (chronomètre). */
  checkGoal: (now: number) => void;
  /** Indique que le résumé est arrivé. */
  markDone: () => void;
  /** Vide la file et renvoie son contenu, pour un envoi. */
  takeBatch: () => Keystroke[];
  /** Remet en tête de file un lot dont l'envoi a échoué. */
  requeue: (batch: Keystroke[]) => void;
  /** Revient à l'état initial. */
  reset: () => void;
}

/** Temps actif à l'instant `now`, arrondi à la ms. */
export function activeMs(clock: Clock, now: number): number {
  return Math.round(
    clock.accumulated + (clock.runningSince === null ? 0 : now - clock.runningSince),
  );
}

/** État de départ, sans session. */
const initial = {
  session: null,
  engine: null,
  version: 0,
  status: 'typing' as Status,
  clock: { accumulated: 0, runningSince: null },
  nextSeq: 0,
  queue: [],
};

/** Store de la session de copie en cours. */
export const usePracticeStore = create<PracticeState>()((set, get) => ({
  ...initial,

  load: (start) => {
    const { settings } = start;
    // Le miroir applique les règles figées de la session.
    const engine = new TypingEngine(start.graphemes, {
      correction: settings.correction,
      backspaceAllowed: settings.backspaceAllowed,
      lenientPunctuation: settings.lenientPunctuation,
    });
    // Reprise : les frappes déjà enregistrées sont rejouées.
    for (const k of start.keystrokes) {
      engine.apply(
        k.kind === 'delete' ? { kind: 'delete' } : { kind: 'insert', typed: k.typed ?? '' },
      );
    }
    // Le temps et la numérotation reprennent là où la session s'était arrêtée.
    const last = start.keystrokes.at(-1);
    set({
      ...initial,
      session: start,
      engine,
      clock: { accumulated: start.activeMs, runningSince: null },
      nextSeq: last ? last.seq + 1 : 0,
    });
  },

  input: (key, now) => {
    const { engine, session, status, clock, nextSeq, queue, version } = get();
    // Aucune frappe hors de l'étape « frappe » (pause, fin en cours…).
    if (!engine || !session || status !== 'typing') return;
    // La première frappe après un arrêt relance le chronomètre.
    const running = clock.runningSince === null ? { ...clock, runningSince: now } : clock;
    // Une frappe refusée n'est ni comptée ni envoyée.
    if (!engine.apply(key)) return;
    const tMs = activeMs(running, now);
    // Frappe brute ajoutée à la file d'envoi, saisie normalisée en NFC.
    const keystroke: Keystroke = {
      seq: nextSeq,
      kind: key.kind,
      typed: key.kind === 'insert' ? key.typed.normalize('NFC') : null,
      tMs,
    };
    const done = isGoalReached(session.settings.goal, engine, session.words, tMs);
    set({
      // Objectif atteint : le chronomètre s'arrête sur la dernière frappe.
      clock: done ? { accumulated: tMs, runningSince: null } : running,
      nextSeq: nextSeq + 1,
      queue: [...queue, keystroke],
      version: version + 1,
      status: done ? 'finishing' : 'typing',
    });
  },

  togglePause: (now) => {
    const { status, clock } = get();
    // La pause fige le temps cumulé ; la reprise attend la prochaine frappe pour relancer.
    if (status === 'typing') {
      set({ status: 'paused', clock: { accumulated: activeMs(clock, now), runningSince: null } });
    } else if (status === 'paused') {
      set({ status: 'typing' });
    }
  },

  checkGoal: (now) => {
    const { engine, session, status, clock } = get();
    if (!engine || !session || status !== 'typing') return;
    // Vérifié régulièrement pour une session chronométrée, même sans frappe.
    if (isGoalReached(session.settings.goal, engine, session.words, activeMs(clock, now))) {
      set({
        status: 'finishing',
        clock: { accumulated: activeMs(clock, now), runningSince: null },
      });
    }
  },

  markDone: () => set({ status: 'done' }),

  takeBatch: () => {
    const { queue } = get();
    set({ queue: [] });
    return queue;
  },

  requeue: (batch) => set((s) => ({ queue: [...batch, ...s.queue] })),

  reset: () => set(initial),
}));
