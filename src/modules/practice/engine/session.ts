/**
 * Miroir de `dactylo-core::typing::TypingSession`, réservé à l'affichage immédiat
 * et au bandeau en direct (ADR 0008). Le cœur Rust fait foi : il rejoue les mêmes
 * frappes brutes pour les statistiques et les fautes enregistrées.
 * Vérifié sur les cas partagés `crates/core/tests/fixtures/typing-cases.json`.
 */

import type { SessionGoal, WordSpan } from '@/shared/api';

export interface Rules {
  correction: 'blocking' | 'free';
  backspaceAllowed: boolean;
  lenientPunctuation: boolean;
}

export type RawKey = { kind: 'insert'; typed: string } | { kind: 'delete' };

/** État d'un caractère ; mêmes codes que `Cell::code` côté Rust. */
export const Cell = { Pending: 0, Correct: 1, Corrected: 2, Wrong: 3 } as const;
export type Cell = (typeof Cell)[keyof typeof Cell];

const CELL_CODES = ['.', 'c', 'f', 'x'] as const;

const segmenter = new Intl.Segmenter('fr', { granularity: 'grapheme' });

/** Découpe une saisie en graphèmes (une touche morte donne un seul graphème). */
export function splitGraphemes(text: string): string[] {
  return Array.from(segmenter.segment(text), (s) => s.segment);
}

function isWhitespace(grapheme: string): boolean {
  return grapheme.length > 0 && /^\s+$/u.test(grapheme);
}

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

function foldPunctuation(grapheme: string): string {
  return Array.from(grapheme, (c) => FOLDS[c] ?? c).join('');
}

/** Même règle que `typing::graphemes_match`. */
export function graphemesMatch(expected: string, typed: string, lenient: boolean): boolean {
  const t = typed.normalize('NFC');
  if (expected === t || (isWhitespace(expected) && isWhitespace(t))) return true;
  return lenient && foldPunctuation(expected) === foldPunctuation(t);
}

export interface Counters {
  inserts: number;
  correctInserts: number;
  correctCells: number;
  uncorrected: number;
}

export class TypingEngine {
  readonly text: readonly string[];
  readonly rules: Rules;
  readonly cells: Uint8Array;
  private readonly everWrong: Uint8Array;
  position = 0;
  private counts: Counters = { inserts: 0, correctInserts: 0, correctCells: 0, uncorrected: 0 };

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

  private insert(raw: string): boolean {
    const typed = raw.normalize('NFC');
    if (splitGraphemes(typed).length !== 1) return false;
    const pos = this.position;
    const expected = this.text[pos];
    if (expected === undefined) return false;

    this.counts.inserts += 1;
    if (graphemesMatch(expected, typed, this.rules.lenientPunctuation)) {
      this.counts.correctInserts += 1;
      this.setCell(pos, this.everWrong[pos] ? Cell.Corrected : Cell.Correct);
      this.position += 1;
    } else {
      this.setCell(pos, Cell.Wrong);
      this.everWrong[pos] = 1;
      if (this.rules.correction === 'free') this.position += 1;
    }
    return true;
  }

  private delete(): boolean {
    if (!this.rules.backspaceAllowed) return false;
    const pos = this.position;
    let target = pos;
    // En mode bloquant, l'erreur se trouve sous le curseur.
    if (this.cells[pos] !== Cell.Wrong) {
      if (pos === 0) return false;
      target = pos - 1;
      this.position = target;
    }
    this.setCell(target, Cell.Pending);
    return true;
  }

  private setCell(i: number, next: Cell) {
    const prev = this.cells[i] as Cell;
    const correct = (c: Cell) => c === Cell.Correct || c === Cell.Corrected;
    this.counts.correctCells += Number(correct(next)) - Number(correct(prev));
    this.counts.uncorrected += Number(next === Cell.Wrong) - Number(prev === Cell.Wrong);
    this.cells[i] = next;
  }

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

export interface LiveStats {
  wpm: number;
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
  const minutes = activeMs / 60_000;
  const raw = minutes > 0 ? counters.inserts / 5 / minutes : 0;
  const wpm = minutes > 0 ? Math.max(0, raw - counters.uncorrected / minutes) : 0;
  const accuracy =
    counters.inserts === 0 ? 100 : (100 * counters.correctInserts) / counters.inserts;
  const limit = goalDurationMs(goal);
  const progress =
    limit !== null ? Math.min(1, activeMs / limit) : endPosition > 0 ? position / endPosition : 1;
  return { wpm, accuracy, progress };
}
