/** Tests du panneau Profils (F-50). */

import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import ProfilesPanel from './ProfilesPanel';

/** Espions des commandes de profils. */
const api = {
  listProfiles: vi.fn(),
  currentProfile: vi.fn(),
  createProfile: vi.fn(),
  updateProfile: vi.fn(),
  deleteProfile: vi.fn(),
  switchProfile: vi.fn(),
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

/** Deux profils : le parent (actif) et l'enfant. */
const parent = { id: 'p1', name: 'Parent', keyboardLayout: 'azerty', createdAt: 0 };
const child = { id: 'p2', name: 'Léa', keyboardLayout: 'bepo', createdAt: 1 };

describe('ProfilesPanel', () => {
  beforeEach(() => {
    for (const fn of Object.values(api)) fn.mockReset();
    api.listProfiles.mockReturnValue(ok([parent, child]));
    api.currentProfile.mockReturnValue(ok(parent));
    api.createProfile.mockReturnValue(ok({ ...child, id: 'p3', name: 'Tom' }));
    api.updateProfile.mockImplementation((id: string, name: string, layout: string) =>
      ok({ id, name, keyboardLayout: layout, createdAt: 0 }),
    );
    api.deleteProfile.mockReturnValue(ok(parent));
    api.switchProfile.mockReturnValue(ok(child));
  });

  it('liste les profils, le profil actif signalé', async () => {
    render(<ProfilesPanel />);
    expect(await screen.findByDisplayValue('Léa')).toBeInTheDocument();
    expect(screen.getByText('Profil actif')).toBeInTheDocument();
    expect(screen.getByLabelText('Disposition clavier de « Léa »')).toHaveValue('bepo');
    expect(screen.getAllByRole('button', { name: 'Utiliser ce profil' })).toHaveLength(1);
  });

  it('crée un profil avec sa disposition', async () => {
    render(<ProfilesPanel />);
    await screen.findByDisplayValue('Léa');
    await userEvent.type(screen.getByLabelText('Nouveau profil'), 'Tom');
    await userEvent.selectOptions(screen.getByLabelText('Disposition clavier'), 'qwerty');
    await userEvent.click(screen.getByRole('button', { name: 'Créer le profil' }));
    expect(api.createProfile).toHaveBeenCalledWith('Tom', 'qwerty');
    await waitFor(() => expect(api.listProfiles).toHaveBeenCalledTimes(2));
  });

  it('renomme et change la disposition', async () => {
    render(<ProfilesPanel />);
    const name = await screen.findByDisplayValue('Léa');
    fireEvent.change(name, { target: { value: 'Léa B.' } });
    fireEvent.blur(name);
    expect(api.updateProfile).toHaveBeenCalledWith('p2', 'Léa B.', 'bepo');
    await userEvent.selectOptions(
      screen.getByLabelText('Disposition clavier de « Parent »'),
      'qwerty',
    );
    expect(api.updateProfile).toHaveBeenLastCalledWith('p1', 'Parent', 'qwerty');
  });

  it('supprime un profil après confirmation', async () => {
    render(<ProfilesPanel />);
    await screen.findByDisplayValue('Léa');
    await userEvent.click(screen.getByRole('button', { name: 'Supprimer le profil « Léa »' }));
    expect(screen.getByText('Supprimer « Léa » et toutes ses statistiques ?')).toBeInTheDocument();
    expect(api.deleteProfile).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole('button', { name: 'Supprimer' }));
    expect(api.deleteProfile).toHaveBeenCalledWith('p2');
  });

  it('affiche le refus de supprimer le dernier profil', async () => {
    api.listProfiles.mockReturnValue(ok([parent]));
    api.deleteProfile.mockReturnValue(
      Promise.resolve({
        status: 'error',
        error: { code: 'invalid', message: 'le dernier profil ne peut pas être supprimé' },
      }),
    );
    render(<ProfilesPanel />);
    await screen.findByDisplayValue('Parent');
    await userEvent.click(screen.getByRole('button', { name: 'Supprimer le profil « Parent »' }));
    await userEvent.click(screen.getByRole('button', { name: 'Supprimer' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('dernier profil');
  });
});
