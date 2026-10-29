/** Boîtes de dialogue de fichiers du système (plugin `dialog`) : seul accès aux
 * fichiers de l'application (NF-11, ADR 0024). */

import { open, save } from '@tauri-apps/plugin-dialog';

/** Filtre d'extensions proposé par la boîte de dialogue. */
export interface FileFilter {
  /** Nom affiché (« Sauvegarde Dactylo »). */
  name: string;
  /** Extensions, sans point. */
  extensions: string[];
}

/** Demande où enregistrer un fichier ; `null` si l'utilisateur annule. */
export async function chooseSaveFile(
  defaultPath: string,
  filters: FileFilter[],
): Promise<string | null> {
  return save({ defaultPath, filters });
}

/** Demande un fichier à ouvrir ; `null` si l'utilisateur annule. */
export async function chooseOpenFile(filters: FileFilter[]): Promise<string | null> {
  const chosen = await open({ multiple: false, directory: false, filters });
  return typeof chosen === 'string' ? chosen : null;
}
