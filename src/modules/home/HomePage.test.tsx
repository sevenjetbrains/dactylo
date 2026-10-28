/** Tests de la page d'accueil. */

import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import HomePage from './HomePage';

/** Réponse simulée de `currentProfile`. */
const currentProfile = vi.fn();
/** Réponse simulée de `pendingSession`. */
const pendingSession = vi.fn();

// Client simulé ; `unwrap` reproduit le vrai comportement (rejet en cas d'erreur).
vi.mock('@/shared/api', () => ({
  commands: {
    currentProfile: () => currentProfile(),
    pendingSession: () => pendingSession(),
  },
  unwrap: async (p: Promise<{ status: string; data?: unknown; error?: unknown }>) => {
    const r = await p;
    if (r.status === 'error') throw r.error;
    return r.data;
  },
}));

/** Profil par défaut renvoyé par le backend simulé. */
const profile = { id: '1', name: 'Par défaut', keyboardLayout: 'azerty', createdAt: 0 };

/** Rend l'accueil dans un routeur en mémoire (les liens en ont besoin). */
function renderHome() {
  return render(
    <MemoryRouter>
      <HomePage />
    </MemoryRouter>,
  );
}

describe('HomePage', () => {
  // Par défaut : profil chargé, aucune session à reprendre.
  beforeEach(() => {
    currentProfile.mockResolvedValue({ status: 'ok', data: profile });
    pendingSession.mockResolvedValue({ status: 'ok', data: null });
  });

  it('salue le profil courant et propose de commencer', async () => {
    renderHome();
    expect(
      await screen.findByRole('heading', { name: 'Bienvenue, Par défaut' }),
    ).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Commencer' })).toHaveAttribute('href', '/practice');
    expect(screen.getByRole('link', { name: 'Ouvrir la bibliothèque' })).toHaveAttribute(
      'href',
      '/library',
    );
    // Pas de carte de reprise sans session interrompue.
    expect(screen.queryByText('Reprendre la session')).not.toBeInTheDocument();
  });

  it('propose de reprendre une session interrompue (NF-06)', async () => {
    pendingSession.mockResolvedValue({
      status: 'ok',
      data: { sessionId: 's1', title: 'Le loup', progress: 0.42, startedAt: 0 },
    });
    renderHome();
    expect(await screen.findByText('Reprendre la session')).toBeInTheDocument();
    expect(screen.getByText(/« Le loup » s'est arrêtée à 42 %/)).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Reprendre' })).toHaveAttribute('href', '/practice');
  });

  it("affiche l'erreur du backend", async () => {
    currentProfile.mockResolvedValue({ status: 'error', error: new Error('base absente') });
    renderHome();
    expect(await screen.findByRole('alert')).toHaveTextContent('base absente');
  });
});
