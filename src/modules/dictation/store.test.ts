/** Tests du pilotage de la dictée : avancement adaptatif et automatique, pause,
 * répétitions, fins de lecture périmées, reprise. */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { activeMs, clockNow, useDictationStore } from './store';
import { countWords, heardWords } from './words';

/** Espions des commandes appelées par le store. */
const api = {
  speakDictationSegment: vi.fn(),
  stopSpeech: vi.fn(),
};

// Toute commande appelée est redirigée vers l'espion du même nom.
vi.mock('@/shared/api', () => ({
  commands: new Proxy(
    {},
    {
      get:
        (_, name: string) =>
        (...args: unknown[]) =>
          api[name as keyof typeof api](...args),
    },
  ),
  unwrap: async (p: Promise<{ status: string; data?: unknown; error?: unknown }>) => {
    const r = await p;
    if (r.status === 'error') throw r.error;
    return r.data;
  },
}));

/** Réglages de test. */
const settings = {
  voice: null,
  rate: 1,
  pitch: 1,
  volume: 1,
  segmentation: 'group' as const,
  groupSize: 2,
  pauseMs: 1000,
  readPunctuation: false,
  advance: 'adaptive' as 'adaptive' | 'automatic',
  maxRepeats: null as number | null,
  lenientPunctuation: true,
};

/** Dictée de trois segments de 2, 2 et 1 mots. */
function start(overrides: Record<string, unknown> = {}) {
  return {
    sessionId: 'd1',
    textId: 't1',
    title: 'Texte',
    exercise: { index: 0, count: 1 },
    segmentWords: [2, 2, 1],
    totalWords: 5,
    settings,
    typed: '',
    activeMs: 0,
    repeats: [0, 0, 0],
    ...overrides,
  };
}

/** Laisse passer les promesses en attente. */
const flush = () => new Promise((r) => setTimeout(r, 0));

describe('comptage des mots', () => {
  it('suit les règles du cœur : apostrophe, trait d’union, ponctuation', () => {
    expect(countWords("l'été, peut-être")).toBe(3);
    expect(countWords('  ')).toBe(0);
    expect(countWords('En 1815, M. Myriel')).toBe(4);
    expect(heardWords([2, 2, 1], 1)).toBe(4);
  });
});

describe('store de la dictée', () => {
  let utterance = 0;
  let now = 0;

  beforeEach(() => {
    utterance = 0;
    now = 0;
    clockNow.now = () => now;
    useDictationStore.getState().reset();
    for (const fn of Object.values(api)) fn.mockReset();
    api.speakDictationSegment.mockImplementation((_id: string, index: number, repeat: boolean) =>
      Promise.resolve({
        status: 'ok',
        data: { utterance: ++utterance, index, repeats: repeat ? 1 : 0 },
      }),
    );
    api.stopSpeech.mockResolvedValue({ status: 'ok', data: null });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('adaptatif : le segment suivant attend les mots entendus', async () => {
    const s = useDictationStore.getState();
    s.load(start());
    await s.play(0);
    expect(useDictationStore.getState().status).toBe('speaking');
    s.onSpeechFinished(1);
    expect(useDictationStore.getState().status).toBe('listening');
    // Un mot sur deux : on attend ; deux mots : segment suivant.
    s.setTyped('En');
    expect(api.speakDictationSegment).toHaveBeenCalledTimes(1);
    s.setTyped('En 1815');
    await flush();
    expect(api.speakDictationSegment).toHaveBeenLastCalledWith('d1', 1, false);
    expect(useDictationStore.getState().current).toBe(1);
  });

  it('l’élève en avance : le segment suivant part dès la fin de lecture', async () => {
    const s = useDictationStore.getState();
    s.load(start());
    await s.play(0);
    s.setTyped('En 1815 déjà');
    expect(api.speakDictationSegment).toHaveBeenCalledTimes(1);
    s.onSpeechFinished(1);
    await flush();
    expect(api.speakDictationSegment).toHaveBeenCalledTimes(2);
  });

  it('automatique : le segment suivant après la pause, puis fin de lecture', async () => {
    vi.useFakeTimers();
    const s = useDictationStore.getState();
    s.load(start({ settings: { ...settings, advance: 'automatic' } }));
    await s.play(0);
    s.onSpeechFinished(1);
    vi.advanceTimersByTime(999);
    expect(api.speakDictationSegment).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(1);
    await vi.runAllTimersAsync();
    expect(api.speakDictationSegment).toHaveBeenLastCalledWith('d1', 1, false);
    // Dernier segment lu : plus rien à lire.
    await s.play(2);
    s.onSpeechFinished(utterance);
    expect(useDictationStore.getState().status).toBe('ended');
  });

  it('ignore la fin d’une ancienne lecture', async () => {
    const s = useDictationStore.getState();
    s.load(start());
    await s.play(0);
    await s.play(0, true);
    s.onSpeechFinished(1);
    expect(useDictationStore.getState().status).toBe('speaking');
    s.onSpeechFinished(2);
    expect(useDictationStore.getState().status).toBe('listening');
  });

  it('pause : silence et chronomètre arrêté ; reprise : le segment est relu', async () => {
    const s = useDictationStore.getState();
    s.load(start({ activeMs: 500 }));
    now = 1_000;
    await s.play(0);
    now = 3_000;
    s.togglePause();
    expect(useDictationStore.getState().status).toBe('paused');
    expect(api.stopSpeech).toHaveBeenCalled();
    now = 10_000;
    expect(activeMs(useDictationStore.getState().clock)).toBe(2_500);
    s.togglePause();
    await flush();
    expect(api.speakDictationSegment).toHaveBeenLastCalledWith('d1', 0, false);
    now = 11_000;
    expect(activeMs(useDictationStore.getState().clock)).toBe(3_500);
  });

  it('répéter et segment précédent comptent comme des répétitions', async () => {
    const s = useDictationStore.getState();
    s.load(start());
    s.repeat();
    expect(api.speakDictationSegment).not.toHaveBeenCalled();
    await s.play(1);
    s.repeat();
    await flush();
    expect(api.speakDictationSegment).toHaveBeenLastCalledWith('d1', 1, true);
    expect(useDictationStore.getState().repeats[1]).toBe(1);
    s.previous();
    await flush();
    expect(api.speakDictationSegment).toHaveBeenLastCalledWith('d1', 0, true);
  });

  it('une erreur de lecture est affichée sans bloquer', async () => {
    api.speakDictationSegment.mockResolvedValue({
      status: 'error',
      error: { code: 'invalid', message: 'plus de répétition' },
    });
    const s = useDictationStore.getState();
    s.load(start());
    await s.play(0);
    expect(useDictationStore.getState().error).toBe('plus de répétition');
    expect(useDictationStore.getState().status).toBe('ready');
  });

  it('reprise : repart après les segments déjà tapés', () => {
    useDictationStore.getState().load(start({ typed: 'En 1815, M.', activeMs: 4_000 }));
    const st = useDictationStore.getState();
    // 3 mots tapés : le premier segment (2 mots) est fait, pas le deuxième (4).
    expect(st.current).toBe(0);
    expect(st.status).toBe('ready');
    expect(activeMs(st.clock)).toBe(4_000);
  });
});
