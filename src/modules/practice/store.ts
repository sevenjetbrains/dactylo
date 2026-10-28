import { create } from 'zustand';

import type { Keystroke_Deserialize as Keystroke, SessionStart_Serialize } from '@/shared/api';

import { isGoalReached, TypingEngine, type RawKey } from './engine/session';

export type SessionStart = SessionStart_Serialize;

export type Status = 'typing' | 'paused' | 'finishing' | 'done';

/**
 * Chronomètre du temps actif : il démarre à la première frappe, s'arrête en pause
 * et repart à la frappe suivante (ADR 0009).
 */
interface Clock {
  accumulated: number;
  runningSince: number | null;
}

interface PracticeState {
  session: SessionStart | null;
  engine: TypingEngine | null;
  /** Incrémenté à chaque frappe acceptée : l'affichage s'y abonne. */
  version: number;
  status: Status;
  clock: Clock;
  nextSeq: number;
  /** Frappes pas encore envoyées au backend. */
  queue: Keystroke[];

  load: (start: SessionStart) => void;
  input: (key: RawKey, now: number) => void;
  togglePause: (now: number) => void;
  /** Passe en fin de session si l'objectif est atteint (chronomètre). */
  checkGoal: (now: number) => void;
  markDone: () => void;
  takeBatch: () => Keystroke[];
  requeue: (batch: Keystroke[]) => void;
  reset: () => void;
}

export function activeMs(clock: Clock, now: number): number {
  return Math.round(
    clock.accumulated + (clock.runningSince === null ? 0 : now - clock.runningSince),
  );
}

const initial = {
  session: null,
  engine: null,
  version: 0,
  status: 'typing' as Status,
  clock: { accumulated: 0, runningSince: null },
  nextSeq: 0,
  queue: [],
};

export const usePracticeStore = create<PracticeState>()((set, get) => ({
  ...initial,

  load: (start) => {
    const { settings } = start;
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
    if (!engine || !session || status !== 'typing') return;
    const running = clock.runningSince === null ? { ...clock, runningSince: now } : clock;
    if (!engine.apply(key)) return;
    const tMs = activeMs(running, now);
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
    if (status === 'typing') {
      set({ status: 'paused', clock: { accumulated: activeMs(clock, now), runningSince: null } });
    } else if (status === 'paused') {
      set({ status: 'typing' });
    }
  },

  checkGoal: (now) => {
    const { engine, session, status, clock } = get();
    if (!engine || !session || status !== 'typing') return;
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
