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

  it("affiche l'erreur du backend", async () => {
    currentProfile.mockResolvedValue({ status: 'error', error: new Error('base absente') });
    renderHome();
    expect(await screen.findByRole('alert')).toHaveTextContent('base absente');
  });
});
