/** Panneau « Apparence » de la page Réglages. */

import { useId } from 'react';
import { useTranslation } from 'react-i18next';

import { setLanguage, SUPPORTED_LANGUAGES } from '@/i18n';
import { useThemeStore, type ThemePreference } from '@/shared/theme';

/** Thèmes proposés, dans l'ordre d'affichage. */
const THEMES: ThemePreference[] = ['light', 'dark', 'system'];

/** Choix du thème (clair, sombre, système) et de la langue de l'interface (F-51). */
export function AppearancePanel() {
  const { t, i18n } = useTranslation('settings');
  const id = useId();
  const theme = useThemeStore((s) => s.theme);
  const setTheme = useThemeStore((s) => s.setTheme);

  return (
    <div className="flex flex-col gap-5">
      <ThemeChoice theme={theme} setTheme={setTheme} />
      {/* Langue de l'interface, retenue par l'appareil ; l'arabe viendra plus tard. */}
      <label htmlFor={`${id}-language`} className="flex flex-col gap-1 text-sm">
        {t('appearance.language')}
        <select
          id={`${id}-language`}
          value={i18n.language}
          onChange={(e) => void setLanguage(e.target.value as (typeof SUPPORTED_LANGUAGES)[number])}
          className="h-9 max-w-xs rounded-md border border-border bg-background px-2 outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
          {SUPPORTED_LANGUAGES.map((l) => (
            <option key={l} value={l}>
              {t(`appearance.languages.${l}`)}
            </option>
          ))}
        </select>
      </label>
    </div>
  );
}

/** Boutons radio du thème. */
function ThemeChoice({
  theme,
  setTheme,
}: {
  theme: ThemePreference;
  setTheme: (theme: ThemePreference) => void;
}) {
  const { t } = useTranslation('settings');
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
