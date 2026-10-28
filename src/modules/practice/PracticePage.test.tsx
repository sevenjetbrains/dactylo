/** Tests de la page du mode Copie : session complète, raccourcis et reprise. */

import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import PracticePage from './PracticePage';
import { splitGraphemes } from './engine/session';
import { usePracticeStore } from './store';

/** Espions des commandes utilisées par la page. */
const api = {
  pendingSession: vi.fn(),
  startSession: vi.fn(),
  resumeSession: vi.fn(),
  wrapText: vi.fn(),
  recordKeystrokes: vi.fn(),
  finishSession: vi.fn(),
  abandonSession: vi.fn(),
  nextTextId: vi.fn(),
  getPracticeSettings: vi.fn(),
  savePracticeSettings: vi.fn(),
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

/** Réponse réussie au format tauri-specta. */
const ok = (data: unknown) => Promise.resolve({ status: 'ok', data });

/** Réglages par défaut. */
const settings = {
  fontSize: 20,
  visibleLines: 3,
  activeLine: 'top',
  correction: 'free',
  backspaceAllowed: true,
  showBanner: true,
  lenientPunctuation: true,
  goal: { type: 'wholeText' },
};

/** Démarrage de session sur « le chat », modifiable champ par champ. */
function sessionStart(overrides: Record<string, unknown> = {}) {
  return {
    sessionId: 's1',
    textId: 't1',
    title: 'Le chat',
    exercise: { index: 0, count: 1 },
    graphemes: splitGraphemes('le chat'),
    words: [
      { start: 0, end: 2 },
      { start: 3, end: 7 },
    ],
    settings,
    keystrokes: [],
    activeMs: 0,
    ...overrides,
  };
}

/** Résumé renvoyé par `finishSession` : un mot fautif, pas de moyenne. */
const summary = {
  sessionId: 's1',
  textId: 't1',
  stats: {
    durationMs: 3_000,
    typed: 7,
    errors: 1,
    uncorrected: 1,
    wpmRaw: 28,
    wpmNet: 8,
    cpm: 120,
    accuracy: 85.7,
  },
  errors: [{ expected: 'le', typed: 'lx', errorType: 'substitution', position: 0, word: true }],
  average: null,
  exercise: { index: 0, count: 1 },
  nextExercise: null,
};

/** Rend la page à l'URL donnée. */
function renderPage(entry = '/practice?text=t1') {
  return render(
    <MemoryRouter initialEntries={[entry]}>
      <PracticePage />
    </MemoryRouter>,
  );
}

/** Tape un texte graphème par graphème, comme le navigateur. */
function type(target: Element, text: string) {
  for (const g of splitGraphemes(text)) {
    fireEvent(
      target,
      new InputEvent('beforeinput', { inputType: 'insertText', data: g, cancelable: true }),
    );
  }
}

describe('PracticePage', () => {
  // Store vidé et backend par défaut : pas de reprise, une session, envois acceptés.
  beforeEach(() => {
    usePracticeStore.getState().reset();
    for (const fn of Object.values(api)) fn.mockReset();
    api.pendingSession.mockReturnValue(ok(null));
    api.startSession.mockImplementation(() => ok(sessionStart()));
    api.wrapText.mockReturnValue(ok([0]));
    api.recordKeystrokes.mockImplementation((_id: string, keys: { seq: number }[]) =>
      ok(keys.at(-1)?.seq ?? null),
    );
    api.finishSession.mockReturnValue(ok(summary));
    api.abandonSession.mockReturnValue(ok(null));
    api.nextTextId.mockReturnValue(ok('t2'));
    api.getPracticeSettings.mockReturnValue(ok(settings));
    api.savePracticeSettings.mockImplementation((s: unknown) => ok(s));
  });

  it('joue une session complète puis affiche le résumé (F-32)', async () => {
    renderPage();
    const input = await screen.findByLabelText('Zone de frappe');
    expect(api.startSession).toHaveBeenCalledWith('t1', null);
    expect(screen.getByLabelText('Statistiques en direct')).toBeInTheDocument();

    // « x » à la place de « e », puis le reste juste jusqu'à la fin du texte.
    type(input, 'lx chat');

    expect(await screen.findByRole('heading', { name: 'Session terminée' })).toBeInTheDocument();
    // Toutes les frappes ont été envoyées, dans l'ordre et numérotées sans trou.
    const sent = api.recordKeystrokes.mock.calls.flatMap(
      ([, keys]) => keys as { seq: number; kind: string; typed: string }[],
    );
    expect(sent.map((k) => k.typed).join('')).toBe('lx chat');
    expect(sent.map((k) => k.seq)).toEqual([0, 1, 2, 3, 4, 5, 6]);
    expect(api.finishSession).toHaveBeenCalledWith('s1', expect.any(Number));
    // Le résumé affiche le mot fautif et son type.
    expect(screen.getByText('lx')).toBeInTheDocument();
    expect(screen.getByText('lettre erronée')).toBeInTheDocument();
    expect(screen.getByText(/Première session/)).toBeInTheDocument();
  });

  it('met en pause avec Échap et ignore la frappe pendant la pause (F-07)', async () => {
    renderPage();
    const input = await screen.findByLabelText('Zone de frappe');
    type(input, 'l');
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(await screen.findByText(/En pause/)).toBeInTheDocument();
    expect(input).toBeDisabled();
    expect(usePracticeStore.getState().status).toBe('paused');

    // Second Échap : reprise, et la frappe suivante est acceptée.
    fireEvent.keyDown(window, { key: 'Escape' });
    await waitFor(() => expect(screen.queryByText(/En pause/)).not.toBeInTheDocument());
    type(input, 'e');
    expect(usePracticeStore.getState().engine?.position).toBe(2);
  });

  it('recommence avec Ctrl+R sans recharger la fenêtre (F-07)', async () => {
    renderPage();
    const input = await screen.findByLabelText('Zone de frappe');
    type(input, 'le');
    // Ctrl+R est intercepté (pas de rechargement) et relance une session vierge.
    const event = new KeyboardEvent('keydown', { key: 'r', ctrlKey: true, cancelable: true });
    window.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);
    await waitFor(() => expect(api.startSession).toHaveBeenCalledTimes(2));
    expect(api.abandonSession).toHaveBeenCalledWith('s1');
    await waitFor(() => expect(usePracticeStore.getState().engine?.position).toBe(0));
  });

  it('passe au texte suivant avec Ctrl+Entrée (F-07)', async () => {
    renderPage();
    await screen.findByLabelText('Zone de frappe');
    fireEvent.keyDown(window, { key: 'Enter', ctrlKey: true });
    await waitFor(() => expect(api.startSession).toHaveBeenLastCalledWith('t2', null));
    expect(api.nextTextId).toHaveBeenCalledWith('t1');
  });

  it('change le type de session depuis la barre d’outils et relance le texte (F-06)', async () => {
    renderPage();
    await screen.findByLabelText('Zone de frappe');
    // L'onglet du type en cours est actif.
    expect(screen.getByRole('button', { name: 'Texte entier' })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    await userEvent.click(screen.getByRole('button', { name: '2 min' }));
    // Réglage enregistré à partir des réglages du profil, puis nouvelle session.
    await waitFor(() =>
      expect(api.savePracticeSettings).toHaveBeenCalledWith({
        ...settings,
        goal: { type: 'duration', minutes: 2 },
      }),
    );
    await waitFor(() => expect(api.startSession).toHaveBeenCalledTimes(2));
    expect(api.abandonSession).toHaveBeenCalledWith('s1');
  });

  it('change la correction et met en pause depuis la barre d’outils (F-04, F-07)', async () => {
    renderPage();
    await screen.findByLabelText('Zone de frappe');
    // Le bouton pause met en pause, puis devient « Reprendre la frappe ».
    await userEvent.click(screen.getByRole('button', { name: 'Mettre en pause' }));
    expect(usePracticeStore.getState().status).toBe('paused');
    await userEvent.click(screen.getByRole('button', { name: 'Reprendre la frappe' }));
    expect(usePracticeStore.getState().status).toBe('typing');

    await userEvent.selectOptions(screen.getByLabelText('Correction'), 'blocking');
    await waitFor(() =>
      expect(api.savePracticeSettings).toHaveBeenCalledWith({
        ...settings,
        correction: 'blocking',
      }),
    );
  });

  it('propose de reprendre une session interrompue et rejoue ses frappes (NF-06)', async () => {
    api.pendingSession.mockReturnValue(
      ok({ sessionId: 's0', title: 'Le chat', progress: 0.25, startedAt: 0 }),
    );
    // La session reprise contient déjà « le », tapé en 400 ms.
    api.resumeSession.mockReturnValue(
      ok(
        sessionStart({
          sessionId: 's0',
          keystrokes: [
            { seq: 0, kind: 'insert', typed: 'l', tMs: 0 },
            { seq: 1, kind: 'insert', typed: 'e', tMs: 400 },
          ],
          activeMs: 400,
        }),
      ),
    );
    renderPage('/practice');
    expect(await screen.findByText(/s'est arrêtée à 25 %/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Reprendre' }));

    const input = await screen.findByLabelText('Zone de frappe');
    expect(api.resumeSession).toHaveBeenCalledWith('s0');
    expect(usePracticeStore.getState().engine?.position).toBe(2);
    // La frappe suivante continue la numérotation et le temps.
    type(input, ' ');
    expect(usePracticeStore.getState().queue).toEqual([
      { seq: 2, kind: 'insert', typed: ' ', tMs: 400 },
    ]);
  });

  it('abandonne la session interrompue sur demande', async () => {
    api.pendingSession.mockReturnValue(
      ok({ sessionId: 's0', title: 'Le chat', progress: 0.5, startedAt: 0 }),
    );
    renderPage();
    await userEvent.click(await screen.findByRole('button', { name: 'Recommencer à zéro' }));
    await screen.findByLabelText('Zone de frappe');
    expect(api.abandonSession).toHaveBeenCalledWith('s0');
    expect(api.startSession).toHaveBeenCalledWith('t1', null);
  });

  it('joue l’exercice demandé par l’URL puis enchaîne sur le suivant (F-24)', async () => {
    // Chaque démarrage crée une session distincte, comme le vrai backend.
    let started = 0;
    api.startSession.mockImplementation((_id: string, exercise: number | null) =>
      ok(
        sessionStart({
          sessionId: `s${++started}`,
          exercise: { index: exercise ?? 0, count: 3 },
        }),
      ),
    );
    api.finishSession.mockReturnValue(
      ok({ ...summary, exercise: { index: 1, count: 3 }, nextExercise: 2 }),
    );
    // `exercise=2` dans l'URL : deuxième exercice (rang 1).
    renderPage('/practice?text=t1&exercise=2');
    await screen.findByLabelText('Zone de frappe');
    expect(api.startSession).toHaveBeenCalledWith('t1', 1);
    expect(screen.getByText('Copie · Exercice 2 sur 3')).toBeInTheDocument();
    // Ctrl+R recommence le même exercice.
    fireEvent.keyDown(window, { key: 'r', ctrlKey: true });
    await waitFor(() => expect(api.startSession).toHaveBeenLastCalledWith('t1', 1));
    // Exercice fini : le bouton de l'écran de fin passe au suivant.
    type(await screen.findByLabelText('Zone de frappe'), 'le chat');
    await userEvent.click(
      await screen.findByRole('button', { name: 'Exercice suivant (3 sur 3)' }),
    );
    await waitFor(() => expect(api.startSession).toHaveBeenLastCalledWith('t1', 2));
  });

  it('renvoie vers la bibliothèque quand il n’y a aucun texte', async () => {
    api.startSession.mockReturnValue(
      Promise.resolve({ status: 'error', error: { code: 'notFound', message: 'aucun texte' } }),
    );
    renderPage('/practice');
    expect(await screen.findByText('Aucun texte à recopier')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Ouvrir la bibliothèque' })).toHaveAttribute(
      'href',
      '/library',
    );
  });
});
