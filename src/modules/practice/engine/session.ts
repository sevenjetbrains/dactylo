/**
 * Miroir de `dactylo-core::typing::TypingSession`, réservé à l'affichage immédiat
 * et au bandeau en direct (ADR 0008). Le cœur Rust fait foi : il rejoue les mêmes
 * frappes brutes pour les statistiques et les fautes enregistrées.
 * Vérifié sur les cas partagés `crates/core/tests/fixtures/typing-cases.json`.
 */

import type { SessionGoal, WordSpan } from '@/shared/api';

/** Règles de comparaison ; mêmes champs que `Rules` côté Rust. */
export interface Rules {
  /** Bloquant ou libre (F-04). */
  correction: 'blocking' | 'free';
  /** Retour arrière autorisé ? */
  backspaceAllowed: boolean;
  /** Tolérance typographique (’ / ', « / ", – / -). */
  lenientPunctuation: boolean;
}

/** Frappe brute saisie : un graphème inséré ou un retour arrière. */
export type RawKey = { kind: 'insert'; typed: string } | { kind: 'delete' };

/** État d'un caractère ; mêmes codes que `Cell::code` côté Rust. */
export const Cell = { Pending: 0, Correct: 1, Corrected: 2, Wrong: 3 } as const;
/** Valeur numérique d'un état de caractère. */
export type Cell = (typeof Cell)[keyof typeof Cell];

/** Code lettre de chaque état, indexé par sa valeur. */
const CELL_CODES = ['.', 'c', 'f', 'x'] as const;

/** Découpeur en graphèmes, créé une seule fois. */
const segmenter = new Intl.Segmenter('fr', { granularity: 'grapheme' });

/** Découpe une saisie en graphèmes (une touche morte donne un seul graphème). */
export function splitGraphemes(text: string): string[] {
  return Array.from(segmenter.segment(text), (s) => s.segment);
}

/** Même règle que `text::is_whitespace`. */
function isWhitespace(grapheme: string): boolean {
  return grapheme.length > 0 && /^\s+$/u.test(grapheme);
}

/** Variantes typographiques et leur forme clavier ; même table que `fold_punctuation`. */
const FOLDS: Record<string, string> = {
  '’': "'",
  '‘': "'",
  ʼ: "'",
  '‛': "'",
  '′': "'",
  '«': '"',
  '»': '"',
  '“': '"',
  '”': '"',
  '„': '"',
  '″': '"',
  '–': '-',
  '—': '-',
  '‐': '-',
  '‑': '-',
  '−': '-',
};

/** Ramène chaque variante typographique à sa forme clavier. */
function foldPunctuation(grapheme: string): string {
  return Array.from(grapheme, (c) => FOLDS[c] ?? c).join('');
}

/** Même règle que `typing::graphemes_match`. */
export function graphemesMatch(expected: string, typed: string, lenient: boolean): boolean {
  // Saisie composée en NFC, puis égalité, espaces équivalentes ou tolérance.
  const t = typed.normalize('NFC');
  if (expected === t || (isWhitespace(expected) && isWhitespace(t))) return true;
  return lenient && foldPunctuation(expected) === foldPunctuation(t);
}

/** Compteurs pour les statistiques ; mêmes champs que `Counters` côté Rust. */
export interface Counters {
  /** Insertions acceptées. */
  inserts: number;
  /** Insertions justes. */
  correctInserts: number;
  /** Caractères justes à présent. */
  correctCells: number;
  /** Caractères faux à présent. */
  uncorrected: number;
}

/** Comparaison de frappe côté interface, pour l'affichage immédiat. */
export class TypingEngine {
  /** Graphèmes du texte. */
  readonly text: readonly string[];
  /** Règles de la session. */
  readonly rules: Rules;
  /** État de chaque caractère (tableau typé : rapide sur 10 000 mots, NF-01). */
  readonly cells: Uint8Array;
  /** 1 si le caractère a été faux au moins une fois. */
  private readonly everWrong: Uint8Array;
  /** Indice du prochain graphème à taper. */
  position = 0;
  /** Compteurs tenus à jour à chaque frappe, sans parcourir le texte. */
  private counts: Counters = { inserts: 0, correctInserts: 0, correctCells: 0, uncorrected: 0 };

  /** Crée un moteur sur un texte déjà découpé en graphèmes. */
  constructor(text: readonly string[], rules: Rules) {
    this.text = text;
    this.rules = rules;
    this.cells = new Uint8Array(text.length);
    this.everWrong = new Uint8Array(text.length);
  }

  /** Applique une frappe ; renvoie `false` si elle est refusée (comme `apply` côté Rust). */
  apply(key: RawKey): boolean {
    return key.kind === 'insert' ? this.insert(key.typed) : this.delete();
  }

  /** Insertion à la position courante. */
  private insert(raw: string): boolean {
    // Une frappe doit être un seul graphème ; rien à taper après la fin du texte.
    const typed = raw.normalize('NFC');
    if (splitGraphemes(typed).length !== 1) return false;
    const pos = this.position;
    const expected = this.text[pos];
    if (expected === undefined) return false;

    this.counts.inserts += 1;
    // Juste : « corrigé » si une erreur a eu lieu ici, et on avance.
    if (graphemesMatch(expected, typed, this.rules.lenientPunctuation)) {
      this.counts.correctInserts += 1;
      this.setCell(pos, this.everWrong[pos] ? Cell.Corrected : Cell.Correct);
      this.position += 1;
    } else {
      // Faux : on n'avance qu'en mode libre.
      this.setCell(pos, Cell.Wrong);
      this.everWrong[pos] = 1;
      if (this.rules.correction === 'free') this.position += 1;
    }
    return true;
  }

  /** Retour arrière. */
  private delete(): boolean {
    // Refusé si les réglages l'interdisent.
    if (!this.rules.backspaceAllowed) return false;
    const pos = this.position;
    let target = pos;
    // En mode bloquant, l'erreur se trouve sous le curseur.
    if (this.cells[pos] !== Cell.Wrong) {
      // Sinon on recule d'un caractère ; impossible au début.
      if (pos === 0) return false;
      target = pos - 1;
      this.position = target;
    }
    this.setCell(target, Cell.Pending);
    return true;
  }

  /** Change l'état d'un caractère en mettant à jour les compteurs par différence. */
  private setCell(i: number, next: Cell) {
    const prev = this.cells[i] as Cell;
    const correct = (c: Cell) => c === Cell.Correct || c === Cell.Corrected;
    this.counts.correctCells += Number(correct(next)) - Number(correct(prev));
    this.counts.uncorrected += Number(next === Cell.Wrong) - Number(prev === Cell.Wrong);
    this.cells[i] = next;
  }

  /** Copie des compteurs courants. */
  counters(): Counters {
    return { ...this.counts };
  }

  /** Cellules en codes lettres, pour les cas partagés. */
  cellCodes(): string {
    return Array.from(this.cells, (c) => CELL_CODES[c]).join('');
  }
}

/** Même règle que `SessionGoal::end_position`. */
export function goalEndPosition(goal: SessionGoal, textLength: number, words: WordSpan[]): number {
  if (goal.type === 'words') return words[goal.count - 1]?.end ?? textLength;
  return textLength;
}

/** Même règle que `SessionGoal::duration_ms`. */
export function goalDurationMs(goal: SessionGoal): number | null {
  return goal.type === 'duration' ? goal.minutes * 60_000 : null;
}

/** Même règle que `SessionGoal::is_reached`. */
export function isGoalReached(
  goal: SessionGoal,
  engine: TypingEngine,
  words: WordSpan[],
  activeMs: number,
): boolean {
  const limit = goalDurationMs(goal);
  return (
    engine.position >= goalEndPosition(goal, engine.text.length, words) ||
    (limit !== null && activeMs >= limit)
  );
}

/** Statistiques du bandeau en direct. */
export interface LiveStats {
  /** MPM net. */
  wpm: number;
  /** Précision, en %. */
  accuracy: number;
  /** De 0 à 1. */
  progress: number;
}

/** Affichage en direct (F-05) : MPM net et précision, mêmes formules que `stats::compute`. */
export function liveStats(
  counters: Counters,
  activeMs: number,
  goal: SessionGoal,
  position: number,
  endPosition: number,
): LiveStats {
  // Vitesse nette : brute moins les erreurs restantes, jamais négative ; 0 sans temps écoulé.
  const minutes = activeMs / 60_000;
  const raw = minutes > 0 ? counters.inserts / 5 / minutes : 0;
  const wpm = minutes > 0 ? Math.max(0, raw - counters.uncorrected / minutes) : 0;
  const accuracy =
    counters.inserts === 0 ? 100 : (100 * counters.correctInserts) / counters.inserts;
  // Progression : temps écoulé pour une session chronométrée, sinon position atteinte.
  const limit = goalDurationMs(goal);
  const progress =
    limit !== null ? Math.min(1, activeMs / limit) : endPosition > 0 ? position / endPosition : 1;
  return { wpm, accuracy, progress };
}
