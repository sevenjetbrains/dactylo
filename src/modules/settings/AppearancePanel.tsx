/** Panneau « Apparence » de la page Réglages. */

import { useTranslation } from 'react-i18next';

import { useThemeStore, type ThemePreference } from '@/shared/theme';

/** Thèmes proposés, dans l'ordre d'affichage. */
const THEMES: ThemePreference[] = ['light', 'dark', 'system'];

/** Choix du thème : clair, sombre ou celui du système. */
export function AppearancePanel() {
  const { t } = useTranslation('settings');
  const theme = useThemeStore((s) => s.theme);
  const setTheme = useThemeStore((s) => s.setTheme);

  return (
    <fieldset className="flex flex-col gap-2">
      <legend className="mb-2 text-sm text-muted-foreground">{t('appearance.theme')}</legend>
      {THEMES.map((value) => (
        <label key={value} className="flex items-center gap-2">
          <input
            type="radio"
            name="theme"
            value={value}
            checked={theme === value}
            onChange={() => setTheme(value)}
            className="accent-primary"
          />
          {t(`appearance.themes.${value}`)}
        </label>
      ))}
    </fieldset>
  );
}
