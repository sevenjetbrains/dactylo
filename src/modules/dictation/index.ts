/** Déclaration du module Dictée auprès du registre. */

import { Volume2 } from 'lucide-react';
import { lazy } from 'react';

import type { ModuleManifest } from '@/app/types';

import en from './locales/en.json';
import fr from './locales/fr.json';

/** Dictée (F-10 à F-17). Le texte se choisit par l'URL : `#/dictation?text=<id>`. */
export const dictation: ModuleManifest = {
  id: 'dictation',
  route: { path: 'dictation', component: lazy(() => import('./DictationPage')) },
  // Entre Copie et Bibliothèque, comme sur les maquettes.
  menu: { labelKey: 'menu', icon: Volume2, order: 20 },
  settingsPanel: {
    titleKey: 'profiles.title',
    component: lazy(() => import('./DictationProfilesPanel')),
  },
  locales: { fr, en },
};
