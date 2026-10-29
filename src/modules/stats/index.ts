/** Déclaration du module Statistiques auprès du registre. */

import { ChartNoAxesColumn } from 'lucide-react';
import { lazy } from 'react';

import type { ModuleManifest } from '@/app/types';

import en from './locales/en.json';
import fr from './locales/fr.json';

/** Statistiques : progression, clavier, export et mots difficiles (F-33 à F-35, F-41). */
export const stats: ModuleManifest = {
  id: 'stats',
  route: { path: 'stats', component: lazy(() => import('./StatsPage')) },
  // Après la Révision, comme sur les maquettes.
  menu: { labelKey: 'menu', icon: ChartNoAxesColumn, order: 50 },
  locales: { fr, en },
};
