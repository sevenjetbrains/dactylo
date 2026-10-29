/** Tests de la coquille : carte de profil et changement de profil (F-50). */

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { Shell } from './Shell';

/** Espions des commandes de la coquille. */
const api = {
  currentProfile: vi.fn(),
  listProfiles: vi.fn(),
  switchProfile: vi.fn(),
};

// Commandes redirigées vers les espions.
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
  unwrap: async (p: Promise<{ status: string; data?: unknown }>) => (await p).data,
}));

/** Réponse réussie au format tauri-specta. */
const ok = (data: unknown) => Promise.resolve({ status: 'ok', data });

/** Deux profils. */
const parent = { id: 'p1', name: 'Parent', keyboardLayout: 'azerty', createdAt: 0 };
const child = { id: 'p2', name: 'Léa', keyboardLayout: 'bepo', createdAt: 1 };

describe('Shell', () => {
  beforeEach(() => {
    for (const fn of Object.values(api)) fn.mockReset();
    api.currentProfile.mockReturnValue(ok(parent));
    api.listProfiles.mockReturnValue(ok([parent, child]));
    api.switchProfile.mockReturnValue(ok(child));
  });

  it('la carte de profil ouvre la liste des profils et change de profil', async () => {
    // Le rechargement de la page n'existe pas sous jsdom : il est neutralisé.
    const reload = vi.fn();
    Object.defineProperty(window, 'location', {
      value: { ...window.location, reload, hash: '' },
      configurable: true,
    });
    render(
      <MemoryRouter>
        <Shell modules={[]} />
      </MemoryRouter>,
    );
    const card = await screen.findByRole('button', { name: 'Profil Parent : changer de profil' });
    expect(card).toHaveAttribute('aria-expanded', 'false');
    await userEvent.click(card);
    const menu = await screen.findByRole('menu', { name: 'Changer de profil' });
    expect(menu).toHaveTextContent('Parent');
    expect(screen.getByRole('menuitemradio', { name: 'Parent' })).toHaveAttribute(
      'aria-checked',
      'true',
    );
    expect(screen.getByRole('menuitem', { name: 'Gérer les profils…' })).toHaveAttribute(
      'href',
      '/settings',
    );
    await userEvent.click(await screen.findByRole('menuitemradio', { name: 'Léa' }));
    expect(api.switchProfile).toHaveBeenCalledWith('p2');
    await vi.waitFor(() => expect(reload).toHaveBeenCalled());
    expect(window.location.hash).toBe('#/');
  });

  it('choisir le profil actif ne change rien', async () => {
    render(
      <MemoryRouter>
        <Shell modules={[]} />
      </MemoryRouter>,
    );
    await userEvent.click(
      await screen.findByRole('button', { name: 'Profil Parent : changer de profil' }),
    );
    await userEvent.click(await screen.findByRole('menuitemradio', { name: 'Parent' }));
    expect(api.switchProfile).not.toHaveBeenCalled();
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  });
});
