import { useMemo } from 'react';
import { RouterProvider } from 'react-router';

import { useApplyTheme } from '@/shared/theme';

import { assertValidRegistry } from './menu';
import { ModulesProvider } from './modules-context';
import { modules } from './registry';
import { createAppRouter } from './router';

assertValidRegistry(modules);

export function App() {
  useApplyTheme();
  const router = useMemo(() => createAppRouter(modules), []);
  return (
    <ModulesProvider modules={modules}>
      <RouterProvider router={router} />
    </ModulesProvider>
  );
}
