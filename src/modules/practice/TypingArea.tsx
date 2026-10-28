/** Zone de frappe du mode Copie. */

import { useEffect, useLayoutEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';

import type { PracticeSettings_Serialize as PracticeSettings } from '@/shared/api';
import { cn } from '@/shared/lib/utils';

import { Cell, splitGraphemes, type RawKey, type TypingEngine } from './engine/session';
import { FALLBACK_COLUMNS, firstVisibleLine, lineOf } from './layout';

/** Hauteur de ligne, en multiple de la taille de police. */
const LINE_HEIGHT = 1.6;

/** Propriétés de la zone de frappe. */
interface Props {
  /** Miroir de comparaison, source de l'état de chaque caractère. */
  engine: TypingEngine;
  /** Change à chaque frappe : force le rendu des cellules modifiées. */
  version: number;
  /** Début de chaque ligne (indices de graphèmes), calculé par le cœur. */
  lineStarts: number[];
  /** Réglages d'affichage. */
  settings: Pick<PracticeSettings, 'fontSize' | 'visibleLines' | 'activeLine'>;
  /** Saisie désactivée (pause, fin). */
  disabled: boolean;
  /** Reçoit chaque frappe brute. */
  onKey: (key: RawKey) => void;
  /** Nombre de caractères par ligne, recalculé au redimensionnement. */
  onColumns: (columns: number) => void;
}

/** Caractère affiché : `\n` visible en ↵, espace fausse visible en ·. */
function glyph(grapheme: string, cell: Cell): string {
  if (grapheme === '\n') return '↵';
  if (cell === Cell.Wrong && /^\s$/u.test(grapheme)) return '·';
  return grapheme;
}

/** Classes de chaque état : à taper, juste, corrigé (souligné ambre), faux (rouge). */
const CELL_CLASS: Record<Cell, string> = {
  [Cell.Pending]: 'text-muted-foreground',
  [Cell.Correct]: 'text-foreground',
  [Cell.Corrected]: 'text-foreground underline decoration-amber-500 decoration-2',
  [Cell.Wrong]: 'bg-destructive/15 text-destructive',
};

/**
 * Zone de frappe (F-01 à F-03). La saisie passe par un champ masqué qui écoute
 * `beforeinput` et `compositionend` : les touches mortes AZERTY (ê, ë…) arrivent
 * en un seul graphème. Seules les lignes visibles sont rendues (NF-01).
 */
export function TypingArea({
  engine,
  version,
  lineStarts,
  settings,
  disabled,
  onKey,
  onColumns,
}: Props) {
  const { t } = useTranslation('practice');
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const textRef = useRef<HTMLDivElement>(null);
  const probeRef = useRef<HTMLSpanElement>(null);
  const { fontSize, visibleLines, activeLine: mode } = settings;

  // Police monospace : colonnes = largeur utile ÷ largeur d'un caractère (F-03).
  useEffect(() => {
    const box = textRef.current;
    const probe = probeRef.current;
    if (!box || !probe) return;
    // La sonde contient 10 caractères : sa largeur ÷ 10 donne celle d'un caractère.
    // Une colonne est gardée en réserve pour le curseur ; 10 colonnes au minimum.
    const measure = () => {
      const charWidth = probe.getBoundingClientRect().width / 10;
      const width = box.clientWidth;
      onColumns(
        charWidth > 0 && width > 0
          ? Math.max(10, Math.floor(width / charWidth) - 1)
          : FALLBACK_COLUMNS,
      );
    };
    measure();
    // Nouvelle mesure à chaque redimensionnement (absent sous jsdom).
    if (typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver(measure);
    observer.observe(box);
    return () => observer.disconnect();
  }, [fontSize, onColumns]);
  // Dernière version du rappel, lue par les écouteurs posés une seule fois.
  const onKeyRef = useRef(onKey);
  useEffect(() => {
    onKeyRef.current = onKey;
  }, [onKey]);

  // Écouteurs posés avant le premier affichage : aucune frappe ne peut être perdue.
  useLayoutEffect(() => {
    const input = inputRef.current;
    if (!input) return;
    // Une saisie peut contenir plusieurs graphèmes : une frappe par graphème.
    const feed = (data: string | null) => {
      for (const g of splitGraphemes(data ?? '')) onKeyRef.current({ kind: 'insert', typed: g });
    };
    const onBeforeInput = (e: InputEvent) => {
      switch (e.inputType) {
        // La composition (touche morte, IME) est lue à `compositionend`.
        case 'insertCompositionText':
        case 'insertFromComposition':
          return;
        // Caractère ordinaire : traité ici, sans laisser le champ se remplir.
        case 'insertText':
          if (e.isComposing) return;
          e.preventDefault();
          feed(e.data);
          return;
        // Entrée : un retour à la ligne attendu dans le texte.
        case 'insertLineBreak':
        case 'insertParagraph':
          e.preventDefault();
          feed('\n');
          return;
        default:
          // Collage, glisser-déposer, suppression : ignorés (le retour arrière passe par keydown).
          e.preventDefault();
      }
    };
    // Fin de composition : le caractère composé (ê) arrive en une fois ; le champ est vidé.
    const onCompositionEnd = (e: CompositionEvent) => {
      feed(e.data);
      input.value = '';
    };
    // Sans contenu dans le champ, `beforeinput` n'est pas émis pour Retour arrière.
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Backspace' && !e.isComposing) {
        e.preventDefault();
        onKeyRef.current({ kind: 'delete' });
      }
    };
    // Écouteurs natifs : React ne transmet pas `beforeinput` avec `inputType`.
    input.addEventListener('beforeinput', onBeforeInput);
    input.addEventListener('compositionend', onCompositionEnd);
    input.addEventListener('keydown', onKeyDown);
    return () => {
      input.removeEventListener('beforeinput', onBeforeInput);
      input.removeEventListener('compositionend', onCompositionEnd);
      input.removeEventListener('keydown', onKeyDown);
    };
  }, []);

  // Le focus revient dans la zone dès qu'elle est réactivée.
  useEffect(() => {
    if (!disabled) inputRef.current?.focus();
  }, [disabled]);

  // Lignes à afficher : la ligne active, placée en haut ou au centre (F-03).
  const position = engine.position;
  const active = lineOf(lineStarts, position);
  const first = firstVisibleLine(active, visibleLines, mode);
  const lines = Array.from({ length: visibleLines }, (_, i) => first + i);

  return (
    <div
      className="relative cursor-text rounded-lg border border-border bg-card px-6 py-4"
      onClick={() => inputRef.current?.focus()}
      data-version={version}
    >
      {/* Champ invisible qui reçoit la saisie ; il couvre la zone pour garder le focus au clic. */}
      <textarea
        ref={inputRef}
        aria-label={t('typingInput')}
        disabled={disabled}
        autoComplete="off"
        autoCorrect="off"
        autoCapitalize="off"
        spellCheck={false}
        className="absolute inset-0 h-full w-full resize-none opacity-0"
      />
      {/* Sonde invisible pour mesurer la largeur d'un caractère. */}
      <span
        ref={probeRef}
        aria-hidden
        className="invisible absolute font-mono whitespace-pre"
        style={{ fontSize }}
      >
        0000000000
      </span>
      {/* Texte affiché ; masqué aux lecteurs d'écran, qui suivent le champ de saisie. */}
      <div
        ref={textRef}
        aria-hidden
        className="font-mono whitespace-pre"
        style={{ fontSize, lineHeight: LINE_HEIGHT, height: `${visibleLines * LINE_HEIGHT}em` }}
      >
        {lines.map((line) => {
          // Ligne au-delà de la fin du texte : ligne vide pour garder la hauteur.
          const start = lineStarts[line];
          if (start === undefined || start >= engine.text.length) {
            return <div key={line}>&nbsp;</div>;
          }
          // Un élément par caractère de la ligne ; le curseur souligne la position courante.
          const end = lineStarts[line + 1] ?? engine.text.length;
          const spans = [];
          for (let i = start; i < end; i++) {
            const cell = engine.cells[i] as Cell;
            const grapheme = engine.text[i] ?? '';
            spans.push(
              <span
                key={i}
                data-cell={cell}
                className={cn(
                  CELL_CLASS[cell],
                  i === position && 'border-b-2 border-primary',
                  grapheme === '\n' && cell === Cell.Pending && 'opacity-50',
                )}
              >
                {glyph(grapheme, cell)}
              </span>,
            );
          }
          return <div key={line}>{spans}</div>;
        })}
      </div>
    </div>
  );
}
