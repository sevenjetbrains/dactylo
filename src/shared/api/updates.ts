/** Mises à jour de l'application (plugins `updater` et `process`), à la demande
 * de l'utilisateur seulement (opt-in, ADR 0025). */

import { relaunch } from '@tauri-apps/plugin-process';
import { check } from '@tauri-apps/plugin-updater';

/** Version disponible, prête à être installée. */
export interface AvailableUpdate {
  /** Numéro de version. */
  version: string;
  /** Notes de version, si la release en a. */
  notes: string | null;
  /** Télécharge, installe, puis redémarre l'application. */
  install: () => Promise<void>;
}

/** Interroge le serveur de mises à jour ; `null` si l'application est à jour. */
export async function checkForUpdate(): Promise<AvailableUpdate | null> {
  const update = await check();
  if (!update) return null;
  return {
    version: update.version,
    notes: update.body ?? null,
    install: async () => {
      await update.downloadAndInstall();
      await relaunch();
    },
  };
}
