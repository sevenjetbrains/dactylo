/** Tests du regroupement en lignes du texte extrait par pdf.js. */

import { describe, expect, it } from 'vitest';

import { piecesToLines, type TextPiece } from './pdf';

/** Morceau de texte à l'ordonnée `y`. */
const piece = (str: string, y: number, hasEOL = false): TextPiece => ({
  str,
  transform: [1, 0, 0, 1, 0, y],
  height: 10,
  hasEOL,
});

describe('piecesToLines', () => {
  it('coupe les lignes sur les retours signalés par pdf.js', () => {
    expect(
      piecesToLines([
        piece('En l’année ', 700),
        piece('1872,', 700, true),
        piece('la maison', 688),
      ]),
    ).toEqual(['En l’année 1872,', 'la maison']);
  });

  it('coupe aussi quand l’ordonnée change sans retour signalé', () => {
    expect(piecesToLines([piece('ligne un', 700), piece('ligne deux', 680)])).toEqual([
      'ligne un',
      'ligne deux',
    ]);
  });

  it('garde ensemble les morceaux d’une même ligne et ignore les lignes vides', () => {
    // Léger décalage vertical (exposant, italique) : même ligne.
    expect(
      piecesToLines([piece('Phileas', 700), piece(' Fogg', 701), piece('  ', 650, true)]),
    ).toEqual(['Phileas Fogg']);
    expect(piecesToLines([])).toEqual([]);
  });
});
