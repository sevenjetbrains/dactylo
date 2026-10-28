/** Fournisseur du contexte des modules. */

import type { ReactNode } from 'react';

import { ModulesContext } from './use-modules';
import type { ModuleManifest } from './types';

/** Donne aux modules (ex. Réglages) la liste des modules sans importer le registre. */
export function ModulesProvider({
  modules,
  children,
}: {
  modules: readonly ModuleManifest[];
  children: ReactNode;
}) {
  return <ModulesContext.Provider value={modules}>{children}</ModulesContext.Provider>;
}
