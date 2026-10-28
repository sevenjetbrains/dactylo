/** Accès à la liste des modules depuis n'importe quel composant. */

import { createContext, useContext } from 'react';

import type { ModuleManifest } from './types';

/** Contexte qui porte le registre ; vide par défaut (tests isolés). */
export const ModulesContext = createContext<readonly ModuleManifest[]>([]);

/** Liste des modules enregistrés. */
export function useModules(): readonly ModuleManifest[] {
  return useContext(ModulesContext);
}
