/** Tests de la page Révision : mots du jour, choix de l'exercice, cas vides. */

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import ReviewPage from './ReviewPage';

/** Espions des commandes utilisées par la page. */
const api = {
  reviewOverview: vi.fn(),
  reviewExercise: vi.fn(),
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

/** Page Copie factice : affiche l'exercice reçu dans l'état de navigation. */
function PracticeProbe() {
  const state = useLocation().state as { review?: string } | null;
  return <p>Copie : {state?.review}</p>;
}

/** Rend la page Révision, avec une route Copie factice pour suivre la navigation. */
function renderPage() {
  return render(
    <MemoryRouter initialEntries={['/review']}>
      <Routes>
        <Route path="/review" element={<ReviewPage />} />
        <Route path="/practice" element={<PracticeProbe />} />
      </Routes>
    </MemoryRouter>,
  );
}

/** Bouton « Lancer l'exercice » de la carte n° `index` (0 mélangés, 1 répétés, 2 phrases). */
async function launchButton(index: number): Promise<HTMLElement> {
  const button = (await screen.findAllByRole('button', { name: "Lancer l'exercice" }))[index];
  if (!button) throw new Error(`bouton ${index} absent`);
  return button;
}

describe('ReviewPage', () => {
  // Par défaut : deux mots dus parmi trois suivis.
  beforeEach(() => {
    for (const fn of Object.values(api)) fn.mockReset();
    api.reviewOverview.mockReturnValue(
      ok({ due: ['accueillir', 'œuvre'], tracked: 3, mastered: 5 }),
    );
    api.reviewExercise.mockReturnValue(ok('œuvre accueillir'));
  });

  it('affiche les mots du jour et lance un exercice mélangé dans la Copie (F-43, F-44)', async () => {
    renderPage();
    expect(
      await screen.findByRole('heading', { name: "2 mots à réviser aujourd'hui" }),
    ).toBeInTheDocument();
    expect(screen.getByText('3 suivis · 5 maîtrisés')).toBeInTheDocument();
    expect(screen.getByRole('list', { name: "Mots à réviser aujourd'hui" })).toHaveTextContent(
      'accueillirœuvre',
    );
    await userEvent.click(await launchButton(0));
    expect(api.reviewExercise).toHaveBeenCalledWith({ type: 'shuffled' }, expect.any(Number));
    expect(await screen.findByText('Copie : œuvre accueillir')).toBeInTheDocument();
  });

  it('transmet le nombre de répétitions choisi', async () => {
    renderPage();
    await userEvent.selectOptions(await screen.findByLabelText('Répétitions'), '4');
    await userEvent.click(await launchButton(1));
    expect(api.reviewExercise).toHaveBeenCalledWith(
      { type: 'repeated', times: 4 },
      expect.any(Number),
    );
  });

  it('prévient quand aucune phrase de la bibliothèque ne contient les mots', async () => {
    api.reviewExercise.mockReturnValue(ok(''));
    renderPage();
    await userEvent.click(await launchButton(2));
    expect(await screen.findByRole('status')).toHaveTextContent(/Aucune phrase/);
    expect(screen.queryByText(/Copie :/)).not.toBeInTheDocument();
  });

  it('sans mot dû, propose de s’avancer sur les mots suivis', async () => {
    api.reviewOverview.mockReturnValue(ok({ due: [], tracked: 2, mastered: 0 }));
    renderPage();
    expect(
      await screen.findByRole('heading', { name: "Rien à réviser aujourd'hui" }),
    ).toBeInTheDocument();
    expect(screen.getByText(/vous avancer/)).toBeInTheDocument();
    expect(screen.getAllByRole('button', { name: "Lancer l'exercice" })).toHaveLength(3);
  });

  it('sans aucun mot suivi, explique comment un mot devient difficile', async () => {
    api.reviewOverview.mockReturnValue(ok({ due: [], tracked: 0, mastered: 0 }));
    renderPage();
    expect(await screen.findByText("Aucun mot difficile pour l'instant")).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: "Lancer l'exercice" })).not.toBeInTheDocument();
    expect(screen.getByRole('link', { name: "S'entraîner" })).toHaveAttribute('href', '/practice');
  });
});
