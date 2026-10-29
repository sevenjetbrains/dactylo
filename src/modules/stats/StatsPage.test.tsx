/** Tests de la page Statistiques : tableau de bord, clavier, export et mots difficiles. */

import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import StatsPage from './StatsPage';

/** Espions des commandes utilisées par la page. */
const api = {
  statsDashboard: vi.fn(),
  keyReport: vi.fn(),
  exportSessionsCsv: vi.fn(),
  difficultWords: vi.fn(),
  reviewOverview: vi.fn(),
  addWord: vi.fn(),
  markWordMastered: vi.fn(),
  removeWord: vi.fn(),
};

// Toute commande appelée est redirigée vers l'espion du même nom ; pas d'événements.
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
  events: { domainEventPayload: { listen: () => new Promise(() => {}) } },
  unwrap: async (p: Promise<{ status: string; data?: unknown; error?: unknown }>) => {
    const r = await p;
    if (r.status === 'error') throw r.error;
    return r.data;
  },
}));

/** Réponse réussie au format tauri-specta. */
const ok = (data: unknown) => Promise.resolve({ status: 'ok', data });

/** Tableau de bord : deux sessions, record 55, série de 9 jours. */
const dashboard = {
  points: [
    { startedAt: Date.UTC(2026, 9, 1, 12), wpmNet: 38, accuracy: 95 },
    { startedAt: Date.UTC(2026, 9, 27, 12), wpmNet: 52, accuracy: 97.8 },
  ],
  period: { trainingMs: (31 * 60 + 40) * 60_000, sessions: 2 },
  averageAccuracy: 96.4,
  bestWpmNet: 55,
  streak: { current: 9, best: 14 },
  trend: { wpmNet: 45, wpmNetDelta: null, accuracy: 96.4, accuracyDelta: null },
  week: { trainingMs: 0, sessions: 0 },
};

/** Carte de chaleur : « é » souvent faux, « a » jamais ; un bigramme lent. */
const keys = {
  keys: [
    { key: 'a', presses: 50, errors: 0, avgMs: 150 },
    { key: 'é', presses: 10, errors: 4, avgMs: 300 },
  ],
  slowBigrams: [{ bigram: 'qu', count: 8, avgMs: 182 }],
  totalErrors: 10,
  accentErrors: 4,
};

/** Un mot difficile dû aujourd'hui. */
const word = {
  word: 'accueillir',
  typedForms: ['acceuillir'],
  dominantError: 'transposition',
  errors: 4,
  encounters: 9,
  streak: 1,
  lastErrorAt: 0,
  nextReviewAt: Date.now() - 1_000,
};

/** Rend la page dans un routeur en mémoire (les liens en ont besoin). */
function renderPage() {
  return render(
    <MemoryRouter>
      <StatsPage />
    </MemoryRouter>,
  );
}

describe('StatsPage', () => {
  beforeEach(() => {
    for (const fn of Object.values(api)) fn.mockReset();
    api.statsDashboard.mockReturnValue(ok(dashboard));
    api.keyReport.mockReturnValue(ok(keys));
    api.exportSessionsCsv.mockReturnValue(ok('﻿Date;Texte\r\n'));
    api.difficultWords.mockReturnValue(ok([word]));
    api.reviewOverview.mockReturnValue(ok({ due: ['accueillir'], tracked: 1, mastered: 64 }));
    api.addWord.mockReturnValue(ok('parallèle'));
    api.markWordMastered.mockReturnValue(ok(null));
    api.removeWord.mockReturnValue(ok(null));
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('affiche la progression, les tuiles, le clavier et les bigrammes (F-33, F-34)', async () => {
    renderPage();
    expect(
      await screen.findByText('de 38 à 52 MPM en 2 sessions', { selector: 'p' }),
    ).toBeInTheDocument();
    expect(api.statsDashboard).toHaveBeenCalledWith('month', expect.any(Number));
    // Courbe accessible et ligne du record.
    expect(screen.getByRole('img', { name: 'de 38 à 52 MPM en 2 sessions' })).toBeInTheDocument();
    expect(screen.getByText('Record personnel · 55')).toBeInTheDocument();
    // Tuiles.
    expect(screen.getByText('96,4 %')).toBeInTheDocument();
    expect(screen.getByText('31 h 40')).toBeInTheDocument();
    expect(screen.getByText('9 jours')).toBeInTheDocument();
    // Touche « é » détaillée, touche jamais tapée signalée, part des accents.
    expect(
      screen.getByRole('img', { name: /é : 4 erreurs sur 10 frappes \(40 %\)/ }),
    ).toBeInTheDocument();
    expect(screen.getByRole('img', { name: 'z : pas encore tapée' })).toBeInTheDocument();
    expect(screen.getByText(/concentrent 40 % de vos erreurs/)).toBeInTheDocument();
    expect(screen.getByText('qu')).toBeInTheDocument();
    expect(screen.getByText('182 ms')).toBeInTheDocument();
  });

  it('recharge la période choisie', async () => {
    renderPage();
    await screen.findByText('de 38 à 52 MPM en 2 sessions', { selector: 'p' });
    await userEvent.click(screen.getByRole('button', { name: '7 jours' }));
    await waitFor(() => expect(api.keyReport).toHaveBeenLastCalledWith('week', expect.any(Number)));
    expect(screen.getByRole('button', { name: '7 jours' })).toHaveAttribute('aria-pressed', 'true');
  });

  it('exporte la période en CSV avec des libellés traduits (F-35)', async () => {
    const createUrl = vi.fn(() => 'blob:csv');
    const revokeUrl = vi.fn();
    Object.assign(URL, { createObjectURL: createUrl, revokeObjectURL: revokeUrl });
    const click = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});
    renderPage();
    await screen.findByText('de 38 à 52 MPM en 2 sessions', { selector: 'p' });
    await userEvent.click(screen.getByRole('button', { name: 'Exporter en CSV' }));
    await waitFor(() => expect(click).toHaveBeenCalled());
    const [period, , labels] = api.exportSessionsCsv.mock.calls[0] as [
      string,
      number,
      { headers: string[]; copy: string; decimalComma: boolean },
    ];
    expect(period).toBe('month');
    expect(labels.headers).toHaveLength(10);
    expect(labels.copy).toBe('Copie');
    expect(labels.decimalComma).toBe(true);
    expect(revokeUrl).toHaveBeenCalledWith('blob:csv');
  });

  it('liste les mots difficiles et permet de les marquer maîtrisés ou de les retirer (F-41, F-45)', async () => {
    renderPage();
    const row = (await screen.findByText('accueillir', { selector: 'td' })).closest('tr');
    if (!row) throw new Error('ligne absente');
    const cells = within(row);
    expect(cells.getByText('acceuillir')).toBeInTheDocument();
    expect(cells.getByText('Inversion')).toBeInTheDocument();
    expect(cells.getByText('4 / 9')).toBeInTheDocument();
    expect(cells.getByText("Aujourd'hui")).toBeInTheDocument();
    expect(cells.getByRole('img', { name: '1 réussite(s) de suite sur 3' })).toBeInTheDocument();
    expect(
      screen.getByText("1 à réviser aujourd'hui · 1 suivis · 64 maîtrisés"),
    ).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Réviser le mot' })).toHaveAttribute('href', '/review');

    // Maîtrise manuelle : commande puis rechargement de la liste.
    await userEvent.click(cells.getByRole('button', { name: 'Marquer « accueillir » maîtrisé' }));
    expect(api.markWordMastered).toHaveBeenCalledWith('accueillir');
    await waitFor(() => expect(api.difficultWords).toHaveBeenCalledTimes(2));
    await userEvent.click(cells.getByRole('button', { name: 'Retirer « accueillir »' }));
    expect(api.removeWord).toHaveBeenCalledWith('accueillir');
  });

  it('ajoute un mot saisi à la main (F-45)', async () => {
    renderPage();
    await userEvent.click(await screen.findByRole('button', { name: 'Ajouter un mot' }));
    await userEvent.type(screen.getByLabelText('Mot à ajouter'), 'parallèle{Enter}');
    expect(api.addWord).toHaveBeenCalledWith('parallèle');
    await waitFor(() => expect(screen.queryByLabelText('Mot à ajouter')).not.toBeInTheDocument());
  });

  it('signale une erreur du backend', async () => {
    api.statsDashboard.mockReturnValue(
      Promise.resolve({ status: 'error', error: { code: 'database', message: 'panne' } }),
    );
    renderPage();
    expect(await screen.findByRole('alert')).toHaveTextContent('panne');
  });
});
