/** Mise en page de la zone de frappe : lignes visibles (F-03). */

import type { PracticeSettings_Serialize as PracticeSettings } from '@/shared/api';

/** Colonnes utilisées quand la mise en page est inconnue (tests, premier rendu). */
export const FALLBACK_COLUMNS = 60;

/** Index de la ligne qui contient `position` (recherche dichotomique). */
export function lineOf(lineStarts: number[], position: number): number {
  let lo = 0;
  let hi = lineStarts.length - 1;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if ((lineStarts[mid] ?? 0) <= position) lo = mid;
    else hi = mid - 1;
  }
  return lo;
}

/** Première ligne affichée : la ligne active reste en haut ou au centre (F-03). */
export function firstVisibleLine(
  activeLine: number,
  visibleLines: number,
  mode: PracticeSettings['activeLine'],
): number {
  const offset = mode === 'center' ? Math.floor((visibleLines - 1) / 2) : 0;
  return Math.max(0, activeLine - offset);
}
