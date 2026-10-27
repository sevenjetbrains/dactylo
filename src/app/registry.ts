import { home } from '@/modules/home';
import { settings } from '@/modules/settings';

import type { ModuleManifest } from './types';

/**
 * Registre des modules : ajouter un module = une ligne ici (ADR 0005).
 * Le routeur, le menu, les réglages et i18n sont construits à partir de ce tableau.
 */
export const modules: readonly ModuleManifest[] = [home, settings];
