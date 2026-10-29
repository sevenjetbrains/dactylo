/** Déclaration du module Révision auprès du registre. */

import { RotateCw } from 'lucide-react';
import { lazy } from 'react';

import type { ModuleManifest } from '@/app/types';

import en from './locales/en.json';
import fr from './locales/fr.json';
import { useDueCount } from './use-review-overview';

/** Révision des mots difficiles (F-40 à F-45), avec le nombre de mots du jour au menu. */
export const review: ModuleManifest = {
  id: 'review',
  route: { path: 'review', component: lazy(() => import('./ReviewPage')) },
  // Après la Bibliothèque, comme sur les maquettes.
  menu: { labelKey: 'menu', icon: RotateCw, order: 40, useBadge: useDueCount },
  settingsPanel: {
    titleKey: 'settings.title',
    component: lazy(() => import('./ReviewSettingsPanel')),
  },
  locales: { fr, en },
};
