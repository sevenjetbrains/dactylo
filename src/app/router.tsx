import { createHashRouter, type RouteObject } from 'react-router';

import { Shell } from './Shell';
import type { ModuleManifest } from './types';

export function buildRoutes(modules: readonly ModuleManifest[]): RouteObject[] {
  return [
    {
      path: '/',
      element: <Shell modules={modules} />,
      children: modules.map(({ route: { path, component: Page } }) =>
        path === '' ? { index: true, element: <Page /> } : { path, element: <Page /> },
      ),
    },
  ];
}

// Routage par fragment (#/…) : fonctionne tel quel avec le protocole de Tauri.
export function createAppRouter(modules: readonly ModuleManifest[]) {
  return createHashRouter(buildRoutes(modules));
}
