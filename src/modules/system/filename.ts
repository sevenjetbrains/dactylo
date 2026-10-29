/** Nom de fichier proposé pour une sauvegarde (F-52). */

/** Nom proposé : « dactylo-sauvegarde-2026-10-29.dactylo » (date locale). */
export function backupFilename(now: Date = new Date()): string {
  const pad = (n: number) => n.toString().padStart(2, '0');
  return `dactylo-sauvegarde-${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}.dactylo`;
}
