import { createContext, useContext } from 'react';

import type { ModuleManifest } from './types';

export const ModulesContext = createContext<readonly ModuleManifest[]>([]);

export function useModules(): readonly ModuleManifest[] {
  return useContext(ModulesContext);
}
