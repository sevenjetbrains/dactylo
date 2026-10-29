/** Déclaration du module Profils auprès du registre. */

import { lazy } from 'react';

import type { ModuleManifest } from '@/app/types';

import en from './locales/en.json';
import fr from './locales/fr.json';

/** Profils (F-50) : un panneau de réglages, sans page ni entrée de menu. Le changement
 * rapide de profil se fait par la carte de profil, en bas du menu. */
export const profiles: ModuleManifest = {
  id: 'profiles',
  settingsPanel: { titleKey: 'title', component: lazy(() => import('./ProfilesPanel')) },
  locales: { fr, en },
};
