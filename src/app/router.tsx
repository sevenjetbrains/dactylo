/** Construction du routeur à partir du registre des modules. */

import { createHashRouter, type RouteObject } from 'react-router';

import { Shell } from './Shell';
import type { ModuleManifest } from './types';

/** Une route par module, toutes dans la coquille (menu + contenu). */
export function buildRoutes(modules: readonly ModuleManifest[]): RouteObject[] {
  return [
    {
      path: '/',
      element: <Shell modules={modules} />,
      // Le chemin vide devient la route d'index (accueil).
      children: modules.flatMap(({ route }) => {
        // Module sans page (panneau de réglages seul) : pas de route.
        if (!route) return [];
        const { path, component: Page } = route;
        return [path === '' ? { index: true, element: <Page /> } : { path, element: <Page /> }];
      }),
    },
  ];
}

// Routage par fragment (#/…) : fonctionne tel quel avec le protocole de Tauri.
export function createAppRouter(modules: readonly ModuleManifest[]) {
  return createHashRouter(buildRoutes(modules));
}
