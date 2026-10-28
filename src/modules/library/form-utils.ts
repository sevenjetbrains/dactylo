/** Outils partagés par les formulaires de la bibliothèque. */

import type { Segmentation } from '@/shared/api';

/** Classes d'un champ de saisie. */
export const inputClass =
  'rounded-lg border border-border bg-card px-3 py-2 outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50';

/** Classes d'un libellé placé au-dessus de son champ. */
export const fieldClass = 'flex flex-col gap-1 text-sm';

/** Métadonnées communes à tous les textes, telles que saisies. */
export interface TextMeta {
  /** Titre. */
  title: string;
  /** Auteur ; vide si inconnu. */
  author: string;
  /** Étiquettes séparées par des virgules. */
  tags: string;
  /** Code de langue. */
  language: string;
  /** Découpage en exercices. */
  segmentation: Segmentation;
}

/** Métadonnées d'un nouveau texte : français, texte entier. */
export const emptyMeta: TextMeta = {
  title: '',
  author: '',
  tags: '',
  language: 'fr',
  segmentation: { type: 'whole' },
};

/** Étiquettes saisies séparées par des virgules. */
export function parseTags(raw: string): string[] {
  return raw
    .split(',')
    .map((t) => t.trim())
    .filter(Boolean);
}

/** Auteur saisi ; vide devient `null`. */
export function authorOf(meta: TextMeta): string | null {
  return meta.author.trim() || null;
}

/** Message lisible d'une erreur, qu'elle vienne du backend (`ApiError`) ou d'ailleurs. */
export function messageOf(e: unknown): string {
  return typeof e === 'object' && e !== null && 'message' in e ? String(e.message) : String(e);
}
