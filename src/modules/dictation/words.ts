/**
 * Comptage des mots tapés, miroir de `text::word_spans` du cœur (comme le miroir de
 * frappe de la Copie, ADR 0008) : suites de lettres ou chiffres, reliées par un
 * trait d'union interne ; l'apostrophe sépare (« l'été » → 2 mots).
 */

/** Un mot : lettres ou chiffres, avec traits d'union internes. */
const WORD = /[\p{L}\p{N}][\p{L}\p{N}\p{M}]*(?:-[\p{L}\p{N}][\p{L}\p{N}\p{M}]*)*/gu;

/** Nombre de mots d'un texte tapé. */
export function countWords(text: string): number {
  return text.normalize('NFC').match(WORD)?.length ?? 0;
}

/** Mots entendus jusqu'au segment `current` compris. */
export function heardWords(segmentWords: readonly number[], current: number): number {
  return segmentWords.slice(0, current + 1).reduce((sum, n) => sum + n, 0);
}
