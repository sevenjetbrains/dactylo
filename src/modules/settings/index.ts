import { Settings } from 'lucide-react';
import { lazy } from 'react';

import type { ModuleManifest } from '@/app/types';

import { AppearancePanel } from './AppearancePanel';
import en from './locales/en.json';
import fr from './locales/fr.json';

export const settings: ModuleManifest = {
  id: 'settings',
  route: { path: 'settings', component: lazy(() => import('./SettingsPage')) },
  menu: { labelKey: 'menu', icon: Settings, order: 100 },
  settingsPanel: { titleKey: 'appearance.title', component: AppearancePanel },
  locales: { fr, en },
};
