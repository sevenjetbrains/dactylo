/** Tests du panneau Données : sauvegarde et restauration (F-52). */

import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { backupFilename } from './filename';
import SystemPanel from './SystemPanel';

/** Espions du client : commandes et boîtes de dialogue. */
const api = {
  backupDatabase: vi.fn(),
  restoreDatabase: vi.fn(),
};
const chooseSaveFile = vi.fn();
const chooseOpenFile = vi.fn();
const getUpdateCheck = vi.fn();
const saveUpdateCheck = vi.fn();
const checkForUpdate = vi.fn();

// Client simulé : commandes redirigées vers les espions, boîtes de dialogue simulées.
vi.mock('@/shared/api', () => ({
  commands: {
    backupDatabase: (p: string) => api.backupDatabase(p),
    restoreDatabase: (p: string) => api.restoreDatabase(p),
    getUpdateCheck: () => getUpdateCheck(),
    saveUpdateCheck: (v: boolean) => saveUpdateCheck(v),
  },
  checkForUpdate: () => checkForUpdate(),
  chooseSaveFile: (...a: unknown[]) => chooseSaveFile(...a),
  chooseOpenFile: (...a: unknown[]) => chooseOpenFile(...a),
  unwrap: async (p: Promise<{ status: string; data?: unknown; error?: unknown }>) => {
    const r = await p;
    if (r.status === 'error') throw r.error;
    return r.data;
  },
}));

/** Réponse réussie au format tauri-specta. */
const ok = (data: unknown) => Promise.resolve({ status: 'ok', data });

describe('SystemPanel', () => {
  beforeEach(() => {
    for (const fn of [api.backupDatabase, api.restoreDatabase, chooseSaveFile, chooseOpenFile]) {
      fn.mockReset();
    }
    api.backupDatabase.mockReturnValue(ok(null));
    api.restoreDatabase.mockReturnValue(ok(null));
    getUpdateCheck.mockReturnValue(ok(false));
    saveUpdateCheck.mockImplementation((v: boolean) => ok(v));
    checkForUpdate.mockReset();
  });

  it('mises à jour : opt-in décoché, aucune requête sans demande (ADR 0025)', async () => {
    render(<SystemPanel />);
    const box = await screen.findByRole('checkbox', {
      name: 'Vérifier les mises à jour au démarrage',
    });
    await waitFor(() => expect(box).toBeEnabled());
    expect(box).not.toBeChecked();
    expect(checkForUpdate).not.toHaveBeenCalled();
    await userEvent.click(box);
    expect(saveUpdateCheck).toHaveBeenCalledWith(true);
  });

  it('vérifier maintenant : à jour, ou version proposée à l’installation', async () => {
    checkForUpdate.mockResolvedValueOnce(null);
    render(<SystemPanel />);
    await userEvent.click(screen.getByRole('button', { name: 'Vérifier maintenant' }));
    expect(await screen.findByText('Dactylo est à jour.')).toBeInTheDocument();
    const install = vi.fn().mockResolvedValue(undefined);
    checkForUpdate.mockResolvedValueOnce({ version: '0.2.0', notes: 'Nouveautés', install });
    await userEvent.click(screen.getByRole('button', { name: 'Vérifier maintenant' }));
    expect(await screen.findByText('Nouveautés')).toBeInTheDocument();
    await userEvent.click(
      screen.getByRole('button', { name: 'Installer la version 0.2.0 et redémarrer' }),
    );
    expect(install).toHaveBeenCalled();
  });

  it('une vérification impossible est expliquée', async () => {
    checkForUpdate.mockRejectedValueOnce(new Error('réseau absent'));
    render(<SystemPanel />);
    await userEvent.click(screen.getByRole('button', { name: 'Vérifier maintenant' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('réseau absent');
  });

  it('propose un nom daté et sauvegarde dans le fichier choisi', async () => {
    chooseSaveFile.mockResolvedValue('C:/Sauvegardes/copie.dactylo');
    render(<SystemPanel />);
    await userEvent.click(screen.getByRole('button', { name: 'Sauvegarder…' }));
    expect(chooseSaveFile).toHaveBeenCalledWith(
      expect.stringMatching(/^dactylo-sauvegarde-\d{4}-\d{2}-\d{2}\.dactylo$/),
      [{ name: 'Sauvegarde Dactylo', extensions: ['dactylo', 'db'] }],
    );
    expect(api.backupDatabase).toHaveBeenCalledWith('C:/Sauvegardes/copie.dactylo');
    expect(await screen.findByText(/Sauvegarde enregistrée/)).toHaveTextContent('copie.dactylo');
  });

  it('annuler la boîte de dialogue ne sauvegarde rien', async () => {
    chooseSaveFile.mockResolvedValue(null);
    render(<SystemPanel />);
    await userEvent.click(screen.getByRole('button', { name: 'Sauvegarder…' }));
    expect(api.backupDatabase).not.toHaveBeenCalled();
  });

  it('restaure après confirmation, et affiche le refus d’un mauvais fichier', async () => {
    chooseOpenFile.mockResolvedValue('C:/notes.txt');
    api.restoreDatabase.mockReturnValue(
      Promise.resolve({
        status: 'error',
        error: { code: 'invalid', message: "ce fichier n'est pas une sauvegarde de Dactylo" },
      }),
    );
    render(<SystemPanel />);
    await userEvent.click(screen.getByRole('button', { name: 'Restaurer…' }));
    expect(screen.getByText(/seront remplacées/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Choisir la sauvegarde' }));
    expect(api.restoreDatabase).toHaveBeenCalledWith('C:/notes.txt');
    expect(await screen.findByRole('alert')).toHaveTextContent("n'est pas une sauvegarde");
  });

  it('nom de fichier daté', () => {
    expect(backupFilename(new Date(2026, 9, 29))).toBe('dactylo-sauvegarde-2026-10-29.dactylo');
  });
});
