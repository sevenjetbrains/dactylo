/** Déclaration du module Bibliothèque auprès du registre. */

import { LibraryBig } from 'lucide-react';
import { lazy } from 'react';

import type { ModuleManifest } from '@/app/types';

import en from './locales/en.json';
import fr from './locales/fr.json';

/** Bibliothèque de textes : collage (F-20) ; import PDF et gestion complète en Phase 2. */
export const library: ModuleManifest = {
  id: 'library',
  route: { path: 'library', component: lazy(() => import('./LibraryPage')) },
  menu: { labelKey: 'menu', icon: LibraryBig, order: 10 },
  locales: { fr, en },
};
