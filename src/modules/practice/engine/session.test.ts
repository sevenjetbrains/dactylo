/** Tests du miroir TypeScript de la comparaison de frappe. */

import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import {
  goalEndPosition,
  graphemesMatch,
  isGoalReached,
  liveStats,
  splitGraphemes,
  TypingEngine,
  type RawKey,
  type Rules,
} from './session';

/** Cas partagé ; même format que côté Rust. */
interface Case {
  name: string;
  text: string;
  rules: Rules;
  keys: string[];
  expected: {
    position: number;
    cells: string;
    inserts: number;
    correctInserts: number;
    correctCells: number;
    uncorrected: number;
    rejected: number;
  };
}

// Cas partagés avec le cœur Rust (ADR 0008), lus depuis la racine du dépôt.
const fixture = JSON.parse(
  readFileSync('crates/core/tests/fixtures/typing-cases.json', 'utf8'),
) as { cases: Case[] };

/** Convertit une touche du fichier de cas (`⌫` = retour arrière) en frappe brute. */
const toKey = (k: string): RawKey =>
  k === '⌫' ? { kind: 'delete' } : { kind: 'insert', typed: k };

/** Règles par défaut : mode libre. */
const free: Rules = { correction: 'free', backspaceAllowed: true, lenientPunctuation: true };

describe('miroir TS de TypingSession', () => {
  it.each(fixture.cases.map((c) => [c.name, c] as const))('%s', (_, c) => {
    const engine = new TypingEngine(splitGraphemes(c.text.normalize('NFC')), c.rules);
    // Les frappes sont appliquées en comptant les refusées, puis tout est comparé.
    const rejected = c.keys.filter((k) => !engine.apply(toKey(k))).length;
    expect(engine.position).toBe(c.expected.position);
    expect(engine.cellCodes()).toBe(c.expected.cells);
    expect(rejected).toBe(c.expected.rejected);
    const { inserts, correctInserts, correctCells, uncorrected } = c.expected;
    expect(engine.counters()).toEqual({ inserts, correctInserts, correctCells, uncorrected });
  });

  it('découpe une saisie en graphèmes', () => {
    expect(splitGraphemes('êt')).toEqual(['ê', 't']);
    expect(splitGraphemes('')).toEqual([]);
  });

  it('compare selon les règles', () => {
    expect(graphemesMatch('ê', 'ê', false)).toBe(true);
    expect(graphemesMatch(' ', ' ', false)).toBe(true);
    expect(graphemesMatch('’', "'", false)).toBe(false);
    expect(graphemesMatch('’', "'", true)).toBe(true);
  });

  it('atteint les objectifs de session', () => {
    // Premier mot « ab » tapé.
    const engine = new TypingEngine(splitGraphemes('ab cd'), free);
    const words = [
      { start: 0, end: 2 },
      { start: 3, end: 5 },
    ];
    engine.apply(toKey('a'));
    engine.apply(toKey('b'));
    expect(goalEndPosition({ type: 'words', count: 1 }, 5, words)).toBe(2);
    expect(goalEndPosition({ type: 'words', count: 9 }, 5, words)).toBe(5);
    expect(isGoalReached({ type: 'words', count: 1 }, engine, words, 0)).toBe(true);
    expect(isGoalReached({ type: 'wholeText' }, engine, words, 0)).toBe(false);
    expect(isGoalReached({ type: 'duration', minutes: 1 }, engine, words, 59_999)).toBe(false);
    expect(isGoalReached({ type: 'duration', minutes: 1 }, engine, words, 60_000)).toBe(true);
  });

  it('calcule le bandeau avec les formules du cœur', () => {
    // Mêmes chiffres que le test Rust `uncorrected_errors_lower_net_speed`.
    const counters = { inserts: 100, correctInserts: 90, correctCells: 86, uncorrected: 4 };
    const live = liveStats(counters, 30_000, { type: 'wholeText' }, 50, 200);
    expect(live.wpm).toBe(32);
    expect(live.accuracy).toBe(90);
    expect(live.progress).toBe(0.25);
    expect(live.errors).toBe(10);
    expect(liveStats(counters, 15_000, { type: 'duration', minutes: 1 }, 0, 10).progress).toBe(
      0.25,
    );
    // Sans frappe ni texte : 0 MPM, 100 % de précision, progression complète.
    const empty = { inserts: 0, correctInserts: 0, correctCells: 0, uncorrected: 0 };
    expect(liveStats(empty, 0, { type: 'wholeText' }, 0, 0)).toEqual({
      wpm: 0,
      accuracy: 100,
      progress: 1,
      errors: 0,
    });
  });

  it('traite une frappe en moins de 16 ms sur un texte de 10 000 mots (NF-01)', () => {
    // 10 000 mots ; on mesure le temps moyen des 1 000 premières frappes.
    const text = splitGraphemes('anticonstitutionnellement '.repeat(10_000));
    const engine = new TypingEngine(text, free);
    const start = performance.now();
    for (const g of text.slice(0, 1_000)) engine.apply({ kind: 'insert', typed: g });
    const perKey = (performance.now() - start) / 1_000;
    expect(perKey).toBeLessThan(16);
    expect(engine.position).toBe(1_000);
  });
});
