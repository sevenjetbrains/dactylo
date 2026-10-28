/** Tests du registre de modules, du menu et du routeur. */

import { render, screen } from '@testing-library/react';
import { Bug } from 'lucide-react';
import { createMemoryRouter, RouterProvider } from 'react-router';
import i18n from 'i18next';
import { describe, expect, it, vi } from 'vitest';

import { buildResources } from '@/i18n';

import { assertValidRegistry, buildMenu } from './menu';
import { modules } from './registry';
import { buildRoutes } from './router';
import type { ModuleManifest } from './types';

// Backend simulé : les appels de l'accueil restent en attente, sans effet.
vi.mock('@/shared/api', () => ({
  commands: {
    currentProfile: vi.fn(() => new Promise(() => {})),
    pendingSession: vi.fn(() => new Promise(() => {})),
  },
  unwrap: (p: Promise<unknown>) => p,
}));

/** Module factice « Hello », pour prouver qu'un module s'ajoute en une ligne. */
const hello: ModuleManifest = {
  id: 'hello',
  route: { path: 'hello', component: () => <p>Bonjour depuis Hello</p> },
  menu: { labelKey: 'menu', icon: Bug, order: 50 },
  locales: { fr: { menu: 'Hello' } },
};

describe('registre de modules', () => {
  it('a des identifiants et des chemins uniques', () => {
    expect(() => assertValidRegistry(modules)).not.toThrow();
  });

  it('refuse un module en double', () => {
    expect(() => assertValidRegistry([hello, hello])).toThrow(/double/);
  });

  it('trie le menu par ordre déclaré', () => {
    const menu = buildMenu([...modules, hello]);
    expect(menu.map((e) => e.moduleId)).toEqual([
      'home',
      'practice',
      'library',
      'hello',
      'settings',
    ]);
    expect(menu[0]?.path).toBe('/');
  });

  it('chaque module a ses traductions françaises', () => {
    for (const m of modules) expect(m.locales?.fr, m.id).toBeDefined();
  });

  it('les traductions du module sont chargées dans son espace de noms', () => {
    const resources = buildResources([...modules, hello]);
    expect(resources.fr?.hello).toEqual({ menu: 'Hello' });
    expect(resources.fr?.common).toBeDefined();
  });

  it("un module ajouté au tableau obtient sa route et son entrée de menu (preuve d'extensibilité)", async () => {
    // Le registre réel plus le module factice, ouvert sur sa route.
    i18n.addResourceBundle('fr', 'hello', { menu: 'Hello' });
    const router = createMemoryRouter(buildRoutes([...modules, hello]), {
      initialEntries: ['/hello'],
    });
    render(<RouterProvider router={router} />);

    // Sa page s'affiche et son lien rejoint ceux des modules existants.
    expect(await screen.findByText('Bonjour depuis Hello')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Accueil' })).toHaveAttribute('href', '/');
    expect(screen.getByRole('link', { name: 'Réglages' })).toHaveAttribute('href', '/settings');
    expect(screen.getByRole('link', { name: 'Hello' })).toHaveAttribute('href', '/hello');
  });
});
