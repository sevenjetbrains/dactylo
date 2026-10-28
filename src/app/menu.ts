/** Menu latéral et contrôle du registre. */

import type { ModuleManifest } from './types';

/** Entrée du menu latéral, prête à afficher. */
export interface MenuEntry {
  /** Module, aussi espace de noms du libellé. */
  moduleId: string;
  /** Chemin absolu de la route. */
  path: string;
  /** Clé du libellé. */
  labelKey: string;
  /** Icône. */
  icon: NonNullable<ModuleManifest['menu']>['icon'];
}

/** Entrées du menu, triées par `order`, pour les modules qui en déclarent une. */
export function buildMenu(modules: readonly ModuleManifest[]): MenuEntry[] {
  // Garde les modules qui ont un menu, triés, puis les convertit en entrées.
  return modules
    .flatMap((m) => (m.menu ? [{ module: m, menu: m.menu }] : []))
    .sort((a, b) => a.menu.order - b.menu.order)
    .map(({ module, menu }) => ({
      moduleId: module.id,
      path: `/${module.route.path}`,
      labelKey: menu.labelKey,
      icon: menu.icon,
    }));
}

/** Vérifie l'unicité des identifiants et des chemins ; lève une erreur au démarrage sinon. */
export function assertValidRegistry(modules: readonly ModuleManifest[]): void {
  const ids = new Set<string>();
  const paths = new Set<string>();
  // Le premier doublon rencontré arrête tout.
  for (const m of modules) {
    if (ids.has(m.id)) throw new Error(`Module en double : ${m.id}`);
    if (paths.has(m.route.path)) throw new Error(`Chemin en double : /${m.route.path}`);
    ids.add(m.id);
    paths.add(m.route.path);
  }
}
