/** Téléchargement de l'export CSV (F-35), sans permission fichier (NF-11, ADR 0017). */

import type { TFunction } from 'i18next';

import type { CsvLabels } from '@/shared/api';

/** Libellés traduits envoyés au backend pour construire le fichier. */
export function csvLabels(t: TFunction, language: string): CsvLabels {
  return {
    headers: t('csv.headers', { returnObjects: true }) as string[],
    copy: t('csv.modes.copy'),
    dictation: t('csv.modes.dictation'),
    review: t('csv.modes.review'),
    completed: t('csv.completed'),
    abandoned: t('csv.abandoned'),
    // Virgule décimale pour le français, comme l'attend Excel en français.
    decimalComma: language.startsWith('fr'),
  };
}

/** Nom du fichier : « dactylo-sessions-2026-10-29.csv » (date locale). */
export function csvFilename(now: Date = new Date()): string {
  const pad = (n: number) => n.toString().padStart(2, '0');
  const day = `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
  return `dactylo-sessions-${day}.csv`;
}

/** Propose le fichier au téléchargement par un lien `blob:` temporaire. */
export function downloadCsv(content: string, filename: string): void {
  const url = URL.createObjectURL(new Blob([content], { type: 'text/csv;charset=utf-8' }));
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  document.body.append(link);
  link.click();
  link.remove();
  URL.revokeObjectURL(url);
}
