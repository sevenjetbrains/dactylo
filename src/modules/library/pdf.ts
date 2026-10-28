/**
 * Lecture d'un PDF avec pdf.js (F-21) : nombre de pages, aperçu d'une page et
 * extraction des lignes de texte. Le nettoyage n'est pas fait ici : il vit dans
 * le cœur Rust (commande `previewPdf`, ADR 0011).
 */

import { getDocument, GlobalWorkerOptions } from 'pdfjs-dist';
import workerUrl from 'pdfjs-dist/build/pdf.worker.min.mjs?url';

import type { PdfPage } from '@/shared/api';

// Le décodage du PDF tourne dans un worker, servi par Vite avec l'application (NF-03).
GlobalWorkerOptions.workerSrc = workerUrl;

/** Morceau de texte tel que fourni par pdf.js (champs utiles seulement). */
export interface TextPiece {
  /** Texte du morceau. */
  str: string;
  /** Matrice de transformation ; `transform[5]` est l'ordonnée de la ligne de base. */
  transform: number[];
  /** Hauteur du texte. */
  height: number;
  /** Le morceau est-il suivi d'un retour à la ligne ? */
  hasEOL: boolean;
}

/**
 * Regroupe les morceaux de texte d'une page en lignes. Une ligne se termine quand
 * pdf.js signale un retour (`hasEOL`) ou quand l'ordonnée change nettement.
 */
export function piecesToLines(pieces: readonly TextPiece[]): string[] {
  const lines: string[] = [];
  let current = '';
  let lastY: number | null = null;
  const flush = () => {
    if (current.trim()) lines.push(current.trim());
    current = '';
  };
  for (const piece of pieces) {
    const y = piece.transform[5] ?? 0;
    // Changement de ligne sans `hasEOL` : écart vertical de plus d'une demi-hauteur.
    if (lastY !== null && Math.abs(y - lastY) > Math.max(2, piece.height / 2)) flush();
    current += piece.str;
    lastY = y;
    if (piece.hasEOL) {
      flush();
      lastY = null;
    }
  }
  flush();
  return lines;
}

/** Document PDF ouvert. */
export interface PdfDocument {
  /** Nombre de pages. */
  pageCount: number;
  /** Lignes des pages `first` à `last` (bornes comprises, à partir de 1). */
  extract: (first: number, last: number) => Promise<PdfPage[]>;
  /** Dessine la page `number` dans le canevas, à la largeur donnée en px. */
  render: (number: number, canvas: HTMLCanvasElement, width: number) => Promise<void>;
  /** Libère le document. */
  close: () => Promise<void>;
}

/** Ouvre un PDF à partir de son contenu ; rejette si le fichier n'est pas un PDF lisible. */
export async function openPdf(data: ArrayBuffer): Promise<PdfDocument> {
  // La tâche de chargement garde le worker ; c'est elle qu'on détruit à la fermeture.
  // Ressources annexes servies par l'application (plugin `pdfjsAssets` de vite.config.ts).
  const assets = `${import.meta.env.BASE_URL}pdfjs`;
  const task = getDocument({
    data: new Uint8Array(data),
    standardFontDataUrl: `${assets}/standard_fonts/`,
    cMapUrl: `${assets}/cmaps/`,
    cMapPacked: true,
    wasmUrl: `${assets}/wasm/`,
  });
  const doc = await task.promise;
  return {
    pageCount: doc.numPages,
    extract: async (first, last) => {
      // Pages extraites l'une après l'autre : l'ordre des lignes est celui du document.
      const pages: PdfPage[] = [];
      for (let number = first; number <= last; number++) {
        const page = await doc.getPage(number);
        const content = await page.getTextContent();
        // Les éléments sans `str` (contenu balisé) ne portent pas de texte.
        const pieces = content.items.flatMap((item) => ('str' in item ? [item] : []));
        pages.push({ number, lines: piecesToLines(pieces) });
      }
      return pages;
    },
    render: async (number, canvas, width) => {
      // Échelle choisie pour remplir la largeur demandée.
      const page = await doc.getPage(number);
      const base = page.getViewport({ scale: 1 });
      const viewport = page.getViewport({ scale: width / base.width });
      canvas.width = Math.floor(viewport.width);
      canvas.height = Math.floor(viewport.height);
      await page.render({ canvas, viewport }).promise;
    },
    close: () => task.destroy(),
  };
}
