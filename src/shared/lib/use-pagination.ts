/** Découpage d'une liste déjà chargée en pages, et numéros de page à afficher. */

import { useState } from 'react';

/** État et page courante d'une liste paginée. */
export interface Pagination<T> {
  /** Numéro de la page affichée, à partir de 1. */
  page: number;
  /** Nombre de pages, au moins 1. */
  pageCount: number;
  /** Éléments de la page affichée. */
  pageItems: T[];
  /** Change de page ; la valeur est bornée à [1, pageCount]. */
  setPage: (page: number) => void;
}

/**
 * Pagination côté interface d'une liste complète (`items`), `pageSize` éléments
 * par page. La page reste valide si la liste raccourcit (suppression, filtre).
 */
export function usePagination<T>(items: readonly T[], pageSize: number): Pagination<T> {
  const [requested, setRequested] = useState(1);
  const pageCount = Math.max(1, Math.ceil(items.length / pageSize));
  // Bornée à chaque rendu plutôt qu'à l'écriture : la liste peut avoir changé depuis.
  const page = Math.min(Math.max(1, requested), pageCount);
  const start = (page - 1) * pageSize;
  return {
    page,
    pageCount,
    pageItems: items.slice(start, start + pageSize),
    setPage: (next) => setRequested(Math.min(Math.max(1, next), pageCount)),
  };
}

/** Pages affichées de part et d'autre de la page courante. */
const SIBLINGS = 1;

/** Élément de la rangée de numéros : une page, ou un saut de pages (« … »). */
export type PageSlot = number | 'gap';

/**
 * Numéros à afficher : première et dernière pages, la page courante et ses
 * voisines ; les pages omises sont remplacées par un « … ». Un « … » qui ne
 * cacherait qu'une seule page est remplacé par cette page.
 */
export function pageSlots(page: number, pageCount: number): PageSlot[] {
  // Peu de pages : la rangée la plus large (bornes, voisines, deux « … ») suffit à tout montrer.
  if (pageCount <= 2 * SIBLINGS + 5) return Array.from({ length: pageCount }, (_, i) => i + 1);
  const shown = new Set([1, pageCount]);
  // Fenêtre autour de la page courante, bornée à la liste.
  for (let p = page - SIBLINGS; p <= page + SIBLINGS; p++) {
    if (p >= 1 && p <= pageCount) shown.add(p);
  }
  const sorted = [...shown].sort((a, b) => a - b);
  const slots: PageSlot[] = [];
  // Comble chaque trou : par la page manquante si elle est seule, sinon par « … ».
  for (const p of sorted) {
    const previous = slots.at(-1);
    if (typeof previous === 'number' && p - previous === 2) slots.push(previous + 1);
    else if (typeof previous === 'number' && p - previous > 2) slots.push('gap');
    slots.push(p);
  }
  return slots;
}
