/** Client typé : seul point d'accès au backend depuis l'interface (CLAUDE.md). */
import type { ApiError } from './bindings';

export { commands, events } from './bindings';
export { chooseOpenFile, chooseSaveFile, type FileFilter } from './files';
export { checkForUpdate, type AvailableUpdate } from './updates';
export type * from './bindings';

/** Forme des réponses des commandes faillibles (mode `Result` de tauri-specta). */
export type Result<T, E> = { status: 'ok'; data: T } | { status: 'error'; error: E };

/** Convertit un `Result` tauri-specta en promesse qui rejette avec l'`ApiError`. */
export async function unwrap<T>(call: Promise<Result<T, ApiError>>): Promise<T> {
  const result = await call;
  // Une erreur métier devient un rejet, pour un usage naturel avec try/catch.
  if (result.status === 'error') throw result.error;
  return result.data;
}
