/** Tests de la page d'accueil. */

import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import HomePage from './HomePage';

/** Réponse simulée de `currentProfile`. */
const currentProfile = vi.fn();
/** Réponse simulée de `pendingSession`. */
const pendingSession = vi.fn();
/** Réponses simulées de la révision et des statistiques. */
const reviewOverview = vi.fn();
const reviewExercise = vi.fn();
const statsDashboard = vi.fn();
const recentSessions = vi.fn();

// Client simulé ; `unwrap` reproduit le vrai comportement (rejet en cas d'erreur).
vi.mock('@/shared/api', () => ({
  commands: {
    currentProfile: () => currentProfile(),
    pendingSession: () => pendingSession(),
    reviewOverview: () => reviewOverview(),
    reviewExercise: (...args: unknown[]) => reviewExercise(...args),
    statsDashboard: () => statsDashboard(),
    recentSessions: () => recentSessions(),
  },
  events: { domainEventPayload: { listen: () => new Promise(() => {}) } },
  unwrap: async (p: Promise<{ status: string; data?: unknown; error?: unknown }>) => {
    const r = await p;
    if (r.status === 'error') throw r.error;
    return r.data;
  },
}));

/** Profil par défaut renvoyé par le backend simulé. */
const profile = { id: '1', name: 'Par défaut', keyboardLayout: 'azerty', createdAt: 0 };

/** Tableau de bord simulé : tendance sur 20 sessions, série de 9 jours. */
const dashboard = {
  points: [],
  period: { trainingMs: 0, sessions: 0 },
  averageAccuracy: 96.4,
  bestWpmNet: 55,
  streak: { current: 9, best: 14 },
  trend: { wpmNet: 48, wpmNetDelta: 3, accuracy: 96.4, accuracyDelta: 0.8 },
  week: { trainingMs: (2 * 60 + 15) * 60_000, sessions: 11 },
};

/** Page Copie factice : affiche l'exercice reçu dans l'état de navigation. */
function PracticeProbe() {
  const state = useLocation().state as { review?: string } | null;
  return <p>Copie : {state?.review}</p>;
}

/** Rend l'accueil dans un routeur en mémoire (les liens en ont besoin). */
function renderHome() {
  return render(
    <MemoryRouter>
      <Routes>
        <Route path="/" element={<HomePage />} />
        <Route path="/practice" element={<PracticeProbe />} />
      </Routes>
    </MemoryRouter>,
  );
}

describe('HomePage', () => {
  // Par défaut : profil chargé, aucune session à reprendre.
  beforeEach(() => {
    currentProfile.mockResolvedValue({ status: 'ok', data: profile });
    pendingSession.mockResolvedValue({ status: 'ok', data: null });
    reviewOverview.mockResolvedValue({ status: 'ok', data: { due: [], tracked: 0, mastered: 0 } });
    reviewExercise.mockResolvedValue({ status: 'ok', data: 'œuvre accueillir' });
    statsDashboard.mockResolvedValue({ status: 'ok', data: dashboard });
    recentSessions.mockResolvedValue({ status: 'ok', data: [] });
  });

  it('salue le profil courant et propose de commencer', async () => {
    renderHome();
    expect(await screen.findByRole('heading', { name: 'Bonjour Par défaut.' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Commencer' })).toHaveAttribute('href', '/practice');
    expect(screen.getByRole('link', { name: 'Ouvrir la bibliothèque' })).toHaveAttribute(
      'href',
      '/library',
    );
    // Raccourci d'en-tête vers l'ajout d'un texte.
    expect(screen.getByRole('link', { name: 'Coller un texte' })).toHaveAttribute(
      'href',
      '/library?view=paste',
    );
    expect(screen.getByRole('link', { name: 'Importer un PDF' })).toHaveAttribute(
      'href',
      '/library?view=pdf',
    );
    // Pas de carte de reprise sans session interrompue.
    expect(screen.queryByText('Reprendre la copie')).not.toBeInTheDocument();
  });

  it('propose de reprendre une session interrompue (NF-06)', async () => {
    pendingSession.mockResolvedValue({
      status: 'ok',
      data: {
        sessionId: 's1',
        textId: 't1',
        title: 'Le loup',
        exercise: { index: 2, count: 8 },
        progress: 0.42,
        startedAt: 0,
      },
    });
    renderHome();
    // Carte de reprise : titre du texte, avancement et bouton Continuer.
    expect(await screen.findByText('Reprendre la copie')).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Le loup' })).toBeInTheDocument();
    expect(screen.getByText('Exercice 3 sur 8')).toBeInTheDocument();
    expect(screen.getByText('42 %')).toBeInTheDocument();
    expect(screen.getByRole('progressbar', { name: 'Progression' })).toHaveAttribute(
      'aria-valuenow',
      '42',
    );
    expect(screen.getByRole('link', { name: 'Continuer' })).toHaveAttribute('href', '/practice');
  });

  it('propose les mots du jour et lance la révision en un clic (F-43)', async () => {
    const due = ['accueillir', 'parallèle', 'néanmoins', 'œuvre', 'vieillard', 'évêque', 'fête'];
    reviewOverview.mockResolvedValue({ status: 'ok', data: { due, tracked: 9, mastered: 2 } });
    renderHome();
    expect(
      await screen.findByRole('heading', { name: "7 mots à réviser aujourd'hui" }),
    ).toBeInTheDocument();
    // Cinq pastilles, puis le reste en nombre.
    expect(screen.getByText('vieillard')).toBeInTheDocument();
    expect(screen.queryByText('évêque')).not.toBeInTheDocument();
    expect(screen.getByText('+ 2')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Réviser maintenant' }));
    expect(reviewExercise).toHaveBeenCalledWith({ type: 'shuffled' }, expect.any(Number));
    expect(await screen.findByText('Copie : œuvre accueillir')).toBeInTheDocument();
  });

  it('sans mot difficile, la carte de révision l’explique', async () => {
    renderHome();
    expect(await screen.findByRole('heading', { name: 'Aucun mot difficile' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Ouvrir la révision' })).toHaveAttribute(
      'href',
      '/review',
    );
  });

  it('affiche les tuiles et les sessions récentes', async () => {
    recentSessions.mockResolvedValue({
      status: 'ok',
      data: [
        {
          sessionId: 'a',
          startedAt: Date.now(),
          title: 'Le Tour du monde',
          mode: 'copy',
          wpmNet: 52,
          accuracy: 97.1,
        },
        {
          sessionId: 'b',
          startedAt: Date.now(),
          title: null,
          mode: 'review',
          wpmNet: 44,
          accuracy: 98.2,
        },
      ],
    });
    renderHome();
    expect(await screen.findByText('+3 sur 10 sessions')).toBeInTheDocument();
    expect(screen.getByText('+0,8 pt')).toBeInTheDocument();
    expect(screen.getByText('Record : 14 jours')).toBeInTheDocument();
    expect(screen.getByText('2 h 15')).toBeInTheDocument();
    expect(screen.getByText('11 sessions')).toBeInTheDocument();
    // Tableau : une copie et une révision (titre générique), avec MPM et précision.
    const table = await screen.findByRole('table');
    const rows = within(table).getAllByRole('row');
    expect(rows).toHaveLength(3);
    expect(within(rows[1] as HTMLElement).getByText('Le Tour du monde')).toBeInTheDocument();
    expect(within(rows[1] as HTMLElement).getByText('97,1 %')).toBeInTheDocument();
    expect(
      within(rows[2] as HTMLElement).getByText('Révision des mots difficiles'),
    ).toBeInTheDocument();
    expect(within(rows[2] as HTMLElement).getByText('Révision')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Tout voir' })).toHaveAttribute('href', '/stats');
  });

  it("affiche l'erreur du backend", async () => {
    currentProfile.mockResolvedValue({ status: 'error', error: new Error('base absente') });
    renderHome();
    expect(await screen.findByRole('alert')).toHaveTextContent('base absente');
  });
});
