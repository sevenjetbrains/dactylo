/** État et pilotage de la dictée en cours (Zustand), F-10 à F-15. */

import { create } from 'zustand';

import {
  commands,
  unwrap,
  type DictationSettings_Serialize as DictationSettings,
  type DictationStart_Serialize as DictationStart,
} from '@/shared/api';

import { countWords, heardWords } from './words';

/**
 * Étape : prête (rien lu), lecture, écoute (segment lu, l'élève tape), pause,
 * tous les segments lus, correction en cours.
 */
export type DictationStatus = 'ready' | 'speaking' | 'listening' | 'paused' | 'ended' | 'finishing';

/** Chronomètre du temps actif : pauses exclues. */
interface Clock {
  /** Début de la période active en cours, ou `null` à l'arrêt. */
  since: number | null;
  /** Temps actif cumulé des périodes précédentes, en ms. */
  accumulated: number;
}

/** État de la dictée. */
interface DictationState {
  /** Dictée chargée, ou `null`. */
  session: DictationStart | null;
  /** Réglages en vigueur (modifiables en cours de route). */
  settings: DictationSettings | null;
  /** Segment en cours, −1 avant la première lecture. */
  current: number;
  /** Numéro de la lecture attendue ; les fins d'autres lectures sont ignorées. */
  utterance: number | null;
  /** Étape. */
  status: DictationStatus;
  /** Texte tapé. */
  typed: string;
  /** Répétitions faites de chaque segment. */
  repeats: number[];
  /** Chronomètre. */
  clock: Clock;
  /** Dernière erreur de lecture, affichée à l'écran. */
  error: string | null;
  /** Charge une dictée (nouvelle ou reprise). */
  load: (start: DictationStart) => void;
  /** Lit le segment `index` ; `repeat` compte une répétition (F-14). */
  play: (index: number, repeat?: boolean) => Promise<void>;
  /** Fin de lecture signalée par le moteur (événement `SpeechFinished`). */
  onSpeechFinished: (utterance: number) => void;
  /** Texte tapé modifié ; peut déclencher le segment suivant (avancement adaptatif). */
  setTyped: (typed: string) => void;
  /** Pause ou reprise (Échap) ; la reprise relit le segment en cours. */
  togglePause: () => void;
  /** Répète le segment en cours (Ctrl+Espace). */
  repeat: () => void;
  /** Relit le segment précédent. */
  previous: () => void;
  /** Remplace les réglages en vigueur. */
  setSettings: (settings: DictationSettings) => void;
  /** Passe en correction : plus de lecture ni de minuteur. */
  markFinishing: () => void;
  /** Oublie la dictée. */
  reset: () => void;
}

/** Minuteur de l'avancement automatique, hors du store (non sérialisable). */
let timer: ReturnType<typeof setTimeout> | null = null;

/** Annule le minuteur d'avancement en cours. */
function clearTimer() {
  if (timer !== null) clearTimeout(timer);
  timer = null;
}

/** Heure courante du chronomètre (surchargeable dans les tests). */
export const clockNow = { now: () => performance.now() };

/** Temps actif de la dictée à l'instant `at`. */
export function activeMs(clock: Clock, at: number = clockNow.now()): number {
  return Math.round(clock.accumulated + (clock.since === null ? 0 : at - clock.since));
}

/** Message lisible d'une erreur de commande. */
function messageOf(e: unknown): string {
  return String((e as { message?: string }).message ?? e);
}

/** État de départ. */
const initial = {
  session: null,
  settings: null,
  current: -1,
  utterance: null,
  status: 'ready' as DictationStatus,
  typed: '',
  repeats: [] as number[],
  clock: { since: null, accumulated: 0 } as Clock,
  error: null,
};

/** Store de la dictée en cours. */
export const useDictationStore = create<DictationState>()((set, get) => {
  /** Avancement adaptatif : le segment suivant dès que l'élève a tout tapé (F-15). */
  const checkAdaptive = () => {
    const { status, settings, session, typed, current } = get();
    if (status !== 'listening' || !session || settings?.advance !== 'adaptive') return;
    if (countWords(typed) >= heardWords(session.segmentWords, current)) advance();
  };

  /** Lit le segment suivant, ou constate la fin de la lecture. */
  const advance = () => {
    clearTimer();
    const { current, session } = get();
    if (!session) return;
    if (current + 1 < session.segmentWords.length) void get().play(current + 1);
    else set({ status: 'ended' });
  };

  return {
    ...initial,

    load: (start) => {
      clearTimer();
      // Une reprise repart du segment qui suit les mots déjà tapés.
      const typedWords = countWords(start.typed);
      let current = -1;
      while (
        start.typed !== '' &&
        current + 1 < start.segmentWords.length &&
        heardWords(start.segmentWords, current + 1) <= typedWords
      ) {
        current++;
      }
      set({
        ...initial,
        session: start,
        settings: start.settings,
        typed: start.typed,
        current,
        repeats: start.repeats,
        clock: { since: null, accumulated: start.activeMs },
      });
    },

    play: async (index, repeat = false) => {
      const { session, clock } = get();
      if (!session) return;
      clearTimer();
      // Le chronomètre démarre à la première lecture, et repart après une pause.
      set({
        status: 'speaking',
        error: null,
        clock: clock.since === null ? { ...clock, since: clockNow.now() } : clock,
      });
      try {
        const spoken = await unwrap(
          commands.speakDictationSegment(session.sessionId, index, repeat),
        );
        // Une autre dictée a pu être chargée entre-temps.
        if (get().session?.sessionId !== session.sessionId) return;
        const repeats = [...get().repeats];
        repeats[index] = spoken.repeats;
        set({ current: index, utterance: spoken.utterance, repeats });
      } catch (e) {
        set({ status: get().current >= 0 ? 'listening' : 'ready', error: messageOf(e) });
      }
    },

    onSpeechFinished: (utterance) => {
      const { status, settings, session, current } = get();
      // Seule la fin de la lecture attendue compte.
      if (status !== 'speaking' || utterance !== get().utterance || !session || !settings) return;
      const last = current + 1 >= session.segmentWords.length;
      set({ status: last ? 'ended' : 'listening' });
      if (last) return;
      // Automatique : après la pause ; adaptatif : quand l'élève a tapé les mots entendus.
      if (settings.advance === 'automatic') timer = setTimeout(advance, settings.pauseMs);
      else checkAdaptive();
    },

    setTyped: (typed) => {
      set({ typed });
      checkAdaptive();
    },

    togglePause: () => {
      const { status, clock, current } = get();
      if (status === 'paused') {
        // Reprise : le chronomètre repart et le segment interrompu est relu.
        set({ clock: { ...clock, since: clockNow.now() }, status: 'listening' });
        if (current >= 0) void get().play(current);
        return;
      }
      if (status !== 'speaking' && status !== 'listening') return;
      clearTimer();
      void unwrap(commands.stopSpeech()).catch(() => undefined);
      set({
        status: 'paused',
        clock: { since: null, accumulated: activeMs(clock) },
      });
    },

    repeat: () => {
      const { current, status } = get();
      if (current >= 0 && status !== 'paused' && status !== 'finishing') {
        void get().play(current, true);
      }
    },

    previous: () => {
      const { current, status } = get();
      if (current >= 0 && status !== 'paused' && status !== 'finishing') {
        void get().play(Math.max(0, current - 1), true);
      }
    },

    setSettings: (settings) => set({ settings }),

    markFinishing: () => {
      clearTimer();
      const { clock } = get();
      set({ status: 'finishing', clock: { since: null, accumulated: activeMs(clock) } });
    },

    reset: () => {
      clearTimer();
      set(initial);
    },
  };
});
