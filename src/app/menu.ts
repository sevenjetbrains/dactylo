import type { ModuleManifest } from './types';

export interface MenuEntry {
  moduleId: string;
  path: string;
  labelKey: string;
  icon: NonNullable<ModuleManifest['menu']>['icon'];
}

/** Entrées du menu, triées par `order`, pour les modules qui en déclarent une. */
export function buildMenu(modules: readonly ModuleManifest[]): MenuEntry[] {
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
  for (const m of modules) {
    if (ids.has(m.id)) throw new Error(`Module en double : ${m.id}`);
    if (paths.has(m.route.path)) throw new Error(`Chemin en double : /${m.route.path}`);
    ids.add(m.id);
    paths.add(m.route.path);
  }
}
