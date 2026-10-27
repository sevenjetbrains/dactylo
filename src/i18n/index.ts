import i18n, { type Resource } from 'i18next';
import { initReactI18next } from 'react-i18next';

import type { ModuleManifest } from '@/app/types';

import commonEn from './locales/en/common.json';
import commonFr from './locales/fr/common.json';

export const SUPPORTED_LANGUAGES = ['fr', 'en'] as const;
export const DEFAULT_LANGUAGE = 'fr';

/** Ressources : espace `common` + un espace par module, nommé par son `id`. */
export function buildResources(modules: readonly ModuleManifest[]): Resource {
  const resources: Resource = { fr: { common: commonFr }, en: { common: commonEn } };
  for (const m of modules) {
    for (const [lang, strings] of Object.entries(m.locales ?? {})) {
      resources[lang] = { ...resources[lang], [m.id]: strings };
    }
  }
  return resources;
}

export function initI18n(modules: readonly ModuleManifest[]) {
  // Initialisation synchrone : les ressources sont embarquées (NF-03).
  void i18n.use(initReactI18next).init({
    resources: buildResources(modules),
    lng: DEFAULT_LANGUAGE,
    fallbackLng: DEFAULT_LANGUAGE,
    defaultNS: 'common',
    interpolation: { escapeValue: false },
    initAsync: false,
  });
  document.documentElement.lang = i18n.language;
  document.documentElement.dir = i18n.dir();
  return i18n;
}
