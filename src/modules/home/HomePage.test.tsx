import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import HomePage from './HomePage';

const currentProfile = vi.fn();

vi.mock('@/shared/api', () => ({
  commands: { currentProfile: () => currentProfile() },
  unwrap: async (p: Promise<{ status: string; data?: unknown; error?: unknown }>) => {
    const r = await p;
    if (r.status === 'error') throw r.error;
    return r.data;
  },
}));

describe('HomePage', () => {
  it('salue le profil courant', async () => {
    currentProfile.mockResolvedValue({
      status: 'ok',
      data: { id: '1', name: 'Par défaut', keyboardLayout: 'azerty', createdAt: 0 },
    });
    render(<HomePage />);
    expect(
      await screen.findByRole('heading', { name: 'Bienvenue, Par défaut' }),
    ).toBeInTheDocument();
  });

  it("affiche l'erreur du backend", async () => {
    currentProfile.mockResolvedValue({ status: 'error', error: new Error('base absente') });
    render(<HomePage />);
    expect(await screen.findByRole('alert')).toHaveTextContent('base absente');
  });
});
