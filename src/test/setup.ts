/** Préparation commune à tous les tests Vitest. */

import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterEach } from 'vitest';

import type { ModuleLocales, ModuleManifest } from '@/app/types';
import { initI18n } from '@/i18n';

/**
 * Traductions de tous les modules, lues directement dans leurs dossiers
 * (`src/modules/<id>/locales/<langue>.json`). Le registre n'est pas importé ici :
 * il chargerait le vrai client `@/shared/api` (pastille du menu) avant que chaque
 * fichier de test ait pu le simuler avec `vi.mock`.
 */
const files = import.meta.glob<Record<string, unknown>>('../modules/*/locales/*.json', {
  eager: true,
  import: 'default',
});
const locales = new Map<string, ModuleLocales>();
for (const [path, strings] of Object.entries(files)) {
  // « ../modules/<id>/locales/<langue>.json » : l'identifiant est le nom du dossier.
  const [, , id = '', , file = ''] = path.split('/');
  const lang = file.replace('.json', '');
  locales.set(id, { ...locales.get(id), [lang]: strings });
}
const modules: ModuleManifest[] = [...locales].map(([id, l]) => ({
  id,
  route: { path: id, component: () => null },
  locales: l,
}));

// Traductions de tous les modules chargées, comme dans l'application.
initI18n(modules);

// Démonte les composants rendus entre deux tests.
afterEach(() => cleanup());
