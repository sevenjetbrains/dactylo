import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import PracticeSettingsPanel from './PracticeSettingsPanel';

const getPracticeSettings = vi.fn();
const savePracticeSettings = vi.fn();

vi.mock('@/shared/api', () => ({
  commands: {
    getPracticeSettings: () => getPracticeSettings(),
    savePracticeSettings: (s: unknown) => savePracticeSettings(s),
  },
  unwrap: async (p: Promise<{ status: string; data?: unknown; error?: unknown }>) => {
    const r = await p;
    if (r.status === 'error') throw r.error;
    return r.data;
  },
}));

const defaults = {
  fontSize: 20,
  visibleLines: 3,
  activeLine: 'top',
  correction: 'free',
  backspaceAllowed: true,
  showBanner: true,
  lenientPunctuation: true,
  goal: { type: 'wholeText' },
};

describe('PracticeSettingsPanel', () => {
  beforeEach(() => {
    getPracticeSettings.mockResolvedValue({ status: 'ok', data: defaults });
    savePracticeSettings.mockImplementation((s: unknown) =>
      Promise.resolve({ status: 'ok', data: s }),
    );
  });

  it('enregistre chaque changement (F-03, F-04, F-05)', async () => {
    render(<PracticeSettingsPanel />);
    const user = userEvent.setup();
    await user.selectOptions(await screen.findByLabelText('Lignes visibles'), '5');
    expect(savePracticeSettings).toHaveBeenLastCalledWith({ ...defaults, visibleLines: 5 });

    await user.click(screen.getByRole('radio', { name: /Bloquante/ }));
    expect(savePracticeSettings).toHaveBeenLastCalledWith(
      expect.objectContaining({ correction: 'blocking' }),
    );

    await user.click(screen.getByRole('checkbox', { name: /bandeau/ }));
    expect(savePracticeSettings).toHaveBeenLastCalledWith(
      expect.objectContaining({ showBanner: false }),
    );

    await user.click(screen.getByRole('radio', { name: 'Au centre' }));
    expect(savePracticeSettings).toHaveBeenLastCalledWith(
      expect.objectContaining({ activeLine: 'center' }),
    );
  });

  it('choisit une session chronométrée ou un nombre de mots (F-06)', async () => {
    render(<PracticeSettingsPanel />);
    const user = userEvent.setup();
    await user.selectOptions(await screen.findByLabelText('Type de session'), 'duration');
    expect(savePracticeSettings).toHaveBeenLastCalledWith(
      expect.objectContaining({ goal: { type: 'duration', minutes: 1 } }),
    );
    await user.selectOptions(screen.getByLabelText('Durée'), '5');
    expect(savePracticeSettings).toHaveBeenLastCalledWith(
      expect.objectContaining({ goal: { type: 'duration', minutes: 5 } }),
    );

    await user.selectOptions(screen.getByLabelText('Type de session'), 'words');
    await waitFor(() => expect(screen.getByLabelText('Nombre de mots')).toHaveValue(50));
  });

  it("affiche l'erreur de chargement", async () => {
    getPracticeSettings.mockResolvedValue({
      status: 'error',
      error: { code: 'database', message: 'base verrouillée' },
    });
    render(<PracticeSettingsPanel />);
    expect(await screen.findByRole('alert')).toHaveTextContent('base verrouillée');
  });
});
