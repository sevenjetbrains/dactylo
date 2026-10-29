/** Tests de la page Réglages. */

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import i18n from 'i18next';
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

  it('change la langue de l’interface et la retient (F-51)', async () => {
    render(
      <ModulesProvider modules={[settings]}>
        <SettingsPage />
      </ModulesProvider>,
    );
    await userEvent.selectOptions(screen.getByLabelText("Langue de l'interface"), 'en');
    expect(i18n.language).toBe('en');
    expect(localStorage.getItem('dactylo-language')).toBe('en');
    expect(document.documentElement.lang).toBe('en');
    // Retour au français pour les autres tests.
    await i18n.changeLanguage('fr');
    localStorage.removeItem('dactylo-language');
  });
});
