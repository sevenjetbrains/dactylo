import { House } from 'lucide-react';
import { lazy } from 'react';

import type { ModuleManifest } from '@/app/types';

import en from './locales/en.json';
import fr from './locales/fr.json';

export const home: ModuleManifest = {
  id: 'home',
  route: { path: '', component: lazy(() => import('./HomePage')) },
  menu: { labelKey: 'menu', icon: House, order: 0 },
  locales: { fr, en },
};
