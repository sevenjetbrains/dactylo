/** Tests de la page Dictée : lecture, copie à l'aveugle, correction, reprise, réglages. */

import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import DictationPage from './DictationPage';
import { useDictationStore } from './store';

/** Espions des commandes utilisées par la page. */
const api = {
  pendingDictation: vi.fn(),
  startDictation: vi.fn(),
  resumeDictation: vi.fn(),
  speakDictationSegment: vi.fn(),
  stopSpeech: vi.fn(),
  saveDictationDraft: vi.fn(),
  finishDictation: vi.fn(),
  abandonDictation: vi.fn(),
  updateDictationSettings: vi.fn(),
  saveDictationSettings: vi.fn(),
  listVoices: vi.fn(),
  listDictationProfiles: vi.fn(),
  wordsExercise: vi.fn(),
  removeWord: vi.fn(),
};

/** Gestionnaire de `SpeechFinished` enregistré par la page. */
let speechFinished: ((e: { payload: { utterance: number } }) => void) | null = null;

// Commandes redirigées vers les espions ; l'événement de fin de lecture est capturé.
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
  events: {
    speechFinished: {
      listen: (h: (e: { payload: { utterance: number } }) => void) => {
        speechFinished = h;
        return Promise.resolve(() => undefined);
      },
    },
  },
  unwrap: async (p: Promise<{ status: string; data?: unknown; error?: unknown }>) => {
    const r = await p;
    if (r.status === 'error') throw r.error;
    return r.data;
  },
}));

/** Réponse réussie au format tauri-specta. */
const ok = (data: unknown) => Promise.resolve({ status: 'ok', data });

/** Réglages de test : adaptatif, groupes de 2. */
const settings = {
  voice: null,
  rate: 1,
  pitch: 1,
  volume: 1,
  segmentation: 'group',
  groupSize: 2,
  pauseMs: 1000,
  readPunctuation: false,
  advance: 'adaptive',
  maxRepeats: null,
  lenientPunctuation: true,
};

/** Démarrage : 3 segments, 5 mots, texte jamais transmis. */
const start = {
  sessionId: 'd1',
  textId: 't1',
  title: 'Les Misérables',
  exercise: { index: 0, count: 1 },
  segmentWords: [2, 2, 1],
  totalWords: 5,
  settings,
  typed: '',
  activeMs: 0,
  repeats: [0, 0, 0],
};

/** Correction : 2 fautes sur 5 mots. */
const summary = {
  sessionId: 'd1',
  textId: 't1',
  title: 'Les Misérables',
  exercise: { index: 0, count: 1 },
  nextExercise: null,
  correction: {
    tokens: [
      { expected: 'En', typed: 'En', word: true, error: null, position: 0, spaceBefore: false },
      { expected: '1815', typed: '1815', word: true, error: null, position: 3, spaceBefore: true },
      {
        expected: 'Monseigneur',
        typed: 'monseigneur',
        word: true,
        error: 'case',
        position: 8,
        spaceBefore: true,
      },
      {
        expected: 'évêque',
        typed: 'évèque',
        word: true,
        error: 'accent',
        position: 20,
        spaceBefore: true,
      },
      {
        expected: 'Digne',
        typed: 'Digne',
        word: true,
        error: null,
        position: 27,
        spaceBefore: true,
      },
    ],
    expectedWords: 5,
    correctWords: 3,
    faults: 2,
    byType: [
      { errorType: 'accent', count: 1 },
      { errorType: 'case', count: 1 },
      { errorType: 'missing_word', count: 0 },
    ],
  },
  stats: { durationMs: 252_000, wpm: 31, accuracy: 60, faults: 2, repeats: 2 },
  average: { sessions: 3, accuracy: 57.9, faults: 7, wpm: 30 },
};

/** Page Copie factice : affiche l'exercice reçu. */
function PracticeProbe() {
  const state = useLocation().state as { review?: string } | null;
  return <p>Copie : {state?.review}</p>;
}

/** Rend la page à l'URL donnée, avec une route Copie factice. */
function renderPage(entry = '/dictation?text=t1') {
  return render(
    <MemoryRouter initialEntries={[entry]}>
      <Routes>
        <Route path="/dictation" element={<DictationPage />} />
        <Route path="/practice" element={<PracticeProbe />} />
      </Routes>
    </MemoryRouter>,
  );
}

/** Simule la fin d'une lecture par le moteur vocal. */
function finishSpeech(utterance: number) {
  act(() => speechFinished?.({ payload: { utterance } }));
}

describe('DictationPage', () => {
  let utterance = 0;

  beforeEach(() => {
    utterance = 0;
    speechFinished = null;
    useDictationStore.getState().reset();
    for (const fn of Object.values(api)) fn.mockReset();
    api.pendingDictation.mockReturnValue(ok(null));
    api.startDictation.mockReturnValue(ok(start));
    api.speakDictationSegment.mockImplementation((_id: string, index: number, repeat: boolean) =>
      ok({ utterance: ++utterance, index, repeats: repeat ? 1 : 0 }),
    );
    api.stopSpeech.mockReturnValue(ok(null));
    api.saveDictationDraft.mockReturnValue(ok(null));
    api.finishDictation.mockReturnValue(ok(summary));
    api.abandonDictation.mockReturnValue(ok(null));
    api.updateDictationSettings.mockImplementation((_id: string, s: unknown) => ok(s));
    api.saveDictationSettings.mockImplementation((s: unknown) => ok(s));
    api.listVoices.mockReturnValue(
      ok([
        { id: 'en', name: 'Zira', language: 'en-US' },
        { id: 'fr', name: 'Hortense', language: 'fr-FR' },
      ]),
    );
    api.listDictationProfiles.mockReturnValue(ok([]));
    api.wordsExercise.mockReturnValue(ok('évêque évêque évêque'));
    api.removeWord.mockReturnValue(ok(null));
  });

  it('lit les segments, avance selon la frappe, puis corrige (F-10, F-15, F-16)', async () => {
    renderPage();
    expect(await screen.findByRole('heading', { name: 'Les Misérables' })).toBeInTheDocument();
    expect(screen.getByText('Dictée · 5 mots')).toBeInTheDocument();
    expect(
      screen.getByText('Texte masqué jusqu’à la correction'.replace('’', "'")),
    ).toBeInTheDocument();
    expect(api.startDictation).toHaveBeenCalledWith('t1', null);
    // Lecture du premier segment, puis fin de lecture : à l'élève de taper.
    await userEvent.click(screen.getByRole('button', { name: 'Commencer la dictée' }));
    expect(api.speakDictationSegment).toHaveBeenCalledWith('d1', 0, false);
    finishSpeech(1);
    expect(await screen.findByText(/à vous de taper/)).toBeInTheDocument();
    // Deux mots tapés : le segment suivant est lu.
    const copy = screen.getByLabelText('Votre copie');
    fireEvent.change(copy, { target: { value: 'En 1815' } });
    await waitFor(() => expect(api.speakDictationSegment).toHaveBeenLastCalledWith('d1', 1, false));
    expect(screen.getByText('Segment 2 sur 3 · lecture en cours')).toBeInTheDocument();
    // Fin : correction avec le texte révélé.
    fireEvent.change(copy, { target: { value: 'En 1815 monseigneur évèque Digne' } });
    await userEvent.click(screen.getByRole('button', { name: 'Terminer et corriger' }));
    expect(api.finishDictation).toHaveBeenCalledWith(
      'd1',
      'En 1815 monseigneur évèque Digne',
      expect.any(Number),
    );
    expect(
      await screen.findByRole('heading', { name: '2 fautes sur 5 mots. Continuez, ça viendra.' }),
    ).toBeInTheDocument();
    expect(screen.getByText('+2,1 pts vs moyenne')).toBeInTheDocument();
    expect(screen.getByText('moyenne : 7')).toBeInTheDocument();
    expect(screen.getByText('4:12')).toBeInTheDocument();
    expect(screen.getByText('2 répétitions')).toBeInTheDocument();
    // Tapé barré, attendu en vert.
    expect(screen.getByText('évèque', { selector: 'del' })).toBeInTheDocument();
    expect(screen.getByText('évêque', { selector: 'ins' })).toBeInTheDocument();
    // Mots ajoutés aux révisions.
    const table = screen.getByRole('table');
    expect(within(table).getAllByRole('row')).toHaveLength(3);
    await userEvent.click(screen.getByRole('button', { name: 'Retirer « évêque » des révisions' }));
    expect(api.removeWord).toHaveBeenCalledWith('évêque');
    expect(await screen.findByText('Retiré')).toBeInTheDocument();
    // Entraînement sur les mots fautifs, dans la Copie.
    await userEvent.click(screen.getByRole('button', { name: "S'entraîner sur ces 2 mots" }));
    expect(api.wordsExercise).toHaveBeenCalledWith(['Monseigneur', 'évêque'], {
      type: 'repeated',
      times: 3,
    });
    expect(await screen.findByText('Copie : évêque évêque évêque')).toBeInTheDocument();
  });

  it('Ctrl+Espace répète, Échap met en pause (F-14)', async () => {
    renderPage();
    await userEvent.click(await screen.findByRole('button', { name: 'Commencer la dictée' }));
    finishSpeech(1);
    fireEvent.keyDown(window, { code: 'Space', key: ' ', ctrlKey: true });
    await waitFor(() => expect(api.speakDictationSegment).toHaveBeenLastCalledWith('d1', 0, true));
    expect(await screen.findByText('Répétitions : 1 / illimitées')).toBeInTheDocument();
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(await screen.findByText(/en pause/)).toBeInTheDocument();
    expect(api.stopSpeech).toHaveBeenCalled();
  });

  it('propose de reprendre une dictée interrompue (NF-06)', async () => {
    api.pendingDictation.mockReturnValue(
      ok({
        sessionId: 'd0',
        textId: 't1',
        title: 'Les Misérables',
        exercise: { index: 0, count: 1 },
        typedWords: 3,
        totalWords: 5,
        startedAt: 0,
      }),
    );
    api.resumeDictation.mockReturnValue(ok({ ...start, sessionId: 'd0', typed: 'En 1815 M' }));
    renderPage('/dictation');
    expect(await screen.findByText(/s'est arrêtée à 3 mots sur 5/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Reprendre' }));
    expect(await screen.findByLabelText('Votre copie')).toHaveValue('En 1815 M');
    expect(screen.getByRole('button', { name: 'Continuer la dictée' })).toBeInTheDocument();
  });

  it('un réglage change la dictée en cours et devient celui du profil', async () => {
    renderPage();
    await userEvent.click(await screen.findByRole('button', { name: 'Commencer la dictée' }));
    const voice = await screen.findByLabelText('Voix');
    // Les voix françaises viennent en premier.
    expect(within(voice).getAllByRole('option')[1]).toHaveTextContent('Hortense');
    await userEvent.selectOptions(voice, 'fr');
    await waitFor(() =>
      expect(api.updateDictationSettings).toHaveBeenCalledWith(
        'd1',
        expect.objectContaining({ voice: 'fr' }),
      ),
    );
    await waitFor(() => expect(api.saveDictationSettings).toHaveBeenCalled());
    // Lecture commencée : le découpage est figé.
    expect(screen.getByRole('button', { name: 'Phrase' })).toBeDisabled();
  });

  it('avant la première lecture, un autre découpage relance la dictée', async () => {
    renderPage();
    await userEvent.click(await screen.findByRole('button', { name: 'Phrase' }));
    await waitFor(() => expect(api.startDictation).toHaveBeenCalledTimes(2));
    expect(api.abandonDictation).toHaveBeenCalledWith('d1', '', 0);
    expect(api.saveDictationSettings).toHaveBeenCalledWith(
      expect.objectContaining({ segmentation: 'sentence' }),
    );
  });

  it('sans texte, renvoie vers la bibliothèque', async () => {
    api.startDictation.mockReturnValue(
      Promise.resolve({ status: 'error', error: { code: 'notFound', message: 'aucun texte' } }),
    );
    renderPage('/dictation');
    expect(await screen.findByText('Aucun texte à dicter')).toBeInTheDocument();
  });
});
