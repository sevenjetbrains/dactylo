/** Tests de la page Réglages. */

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';

import { ModulesProvider } from '@/app/modules-context';
import type { ModuleManifest } from '@/app/types';
import { useThemeStore } from '@/shared/theme';

import { settings } from './index';
import SettingsPage from './SettingsPage';

/** Module factice qui déclare son propre panneau. */
const extra: ModuleManifest = {
  id: 'extra',
  route: { path: 'extra', component: () => null },
  settingsPanel: { titleKey: 'panel', component: () => <p>Réglage supplémentaire</p> },
};

describe('SettingsPage', () => {
  it('affiche les panneaux déclarés par tous les modules', () => {
    render(
      <ModulesProvider modules={[settings, extra]}>
        <SettingsPage />
      </ModulesProvider>,
    );
    // Le panneau du module Réglages et celui du module factice.
    expect(screen.getByRole('heading', { name: 'Apparence' })).toBeInTheDocument();
    expect(screen.getByText('Réglage supplémentaire')).toBeInTheDocument();
  });

  it('change le thème', async () => {
    render(
      <ModulesProvider modules={[settings]}>
        <SettingsPage />
      </ModulesProvider>,
    );
    // Le choix est enregistré dans le store du thème.
    await userEvent.click(screen.getByRole('radio', { name: 'Sombre' }));
    expect(useThemeStore.getState().theme).toBe('dark');
  });
});
