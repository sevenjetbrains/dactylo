/** Déclaration du module Système auprès du registre. */

import { lazy } from 'react';

import type { ModuleManifest } from '@/app/types';

import en from './locales/en.json';
import fr from './locales/fr.json';

/** Données de l'application : sauvegarde et restauration (F-52). Un panneau de
 * réglages, sans page ni entrée de menu. */
export const system: ModuleManifest = {
  id: 'system',
  settingsPanel: { titleKey: 'title', component: lazy(() => import('./SystemPanel')) },
  locales: { fr, en },
};
