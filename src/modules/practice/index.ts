import { Keyboard } from 'lucide-react';
import { lazy } from 'react';

import type { ModuleManifest } from '@/app/types';

import en from './locales/en.json';
import fr from './locales/fr.json';

/** Mode Copie. Le texte se choisit par l'URL : `#/practice?text=<id>`. */
export const practice: ModuleManifest = {
  id: 'practice',
  route: { path: 'practice', component: lazy(() => import('./PracticePage')) },
  menu: { labelKey: 'menu', icon: Keyboard, order: 20 },
  settingsPanel: {
    titleKey: 'settings.title',
    component: lazy(() => import('./PracticeSettingsPanel')),
  },
  locales: { fr, en },
};
