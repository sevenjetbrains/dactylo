/** Composant racine : thème, contexte des modules et routeur. */

import { useMemo } from 'react';
import { RouterProvider } from 'react-router';

import { useApplyTheme } from '@/shared/theme';

import { assertValidRegistry } from './menu';
import { ModulesProvider } from './modules-context';
import { modules } from './registry';
import { createAppRouter } from './router';

// Un registre incohérent (doublon) doit échouer dès le chargement.
assertValidRegistry(modules);

/** Application complète. */
export function App() {
  useApplyTheme();
  // Le routeur n'est construit qu'une fois : le registre ne change pas.
  const router = useMemo(() => createAppRouter(modules), []);
  return (
    <ModulesProvider modules={modules}>
      <RouterProvider router={router} />
    </ModulesProvider>
  );
}
