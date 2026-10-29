/** Registre central des modules de l'application. */

import { home } from '@/modules/home';
import { library } from '@/modules/library';
import { practice } from '@/modules/practice';
import { review } from '@/modules/review';
import { settings } from '@/modules/settings';
import { stats } from '@/modules/stats';

import type { ModuleManifest } from './types';

/**
 * Registre des modules : ajouter un module = une ligne ici (ADR 0005).
 * Le routeur, le menu, les réglages et i18n sont construits à partir de ce tableau.
 */
export const modules: readonly ModuleManifest[] = [
  home,
  library,
  practice,
  review,
  stats,
  settings,
];
