/** Clavier virtuel sous la zone de frappe (F-08, ADR 0022) : touche suivante et
 * doigt conseillé, dans la disposition du profil. */

import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

import {
  commands,
  unwrap,
  type Finger,
  type KeyDef,
  type KeyboardLayout,
  type Press,
} from '@/shared/api';
import { cn } from '@/shared/lib/utils';

/** Couleur de chaque doigt (bord haut des touches, légende), comme sur la maquette. */
const FINGER_COLORS: Record<Finger, string> = {
  leftPinky: 'border-t-violet-500 bg-violet-500',
  leftRing: 'border-t-blue-500 bg-blue-500',
  leftMiddle: 'border-t-teal-500 bg-teal-500',
  leftIndex: 'border-t-green-500 bg-green-500',
  leftThumb: 'border-t-slate-400 bg-slate-400',
  rightThumb: 'border-t-slate-400 bg-slate-400',
  rightIndex: 'border-t-amber-500 bg-amber-500',
  rightMiddle: 'border-t-orange-500 bg-orange-500',
  rightRing: 'border-t-pink-500 bg-pink-500',
  rightPinky: 'border-t-red-500 bg-red-500',
};

/** Largeur maximale du clavier : celle de la maquette sur un grand écran. */
const MAX_WIDTH = '52rem';

/** Légende affichée sur une touche : majuscule pour une lettre, sinon le caractère de base. */
function label(key: KeyDef): string {
  const letter = /^\p{Ll}$/u.test(key.base) && key.shift === key.base.toUpperCase();
  return letter ? (key.shift ?? key.base) : key.base;
}

/** La main d'un doigt (pour choisir la touche Maj opposée). */
function isLeft(finger: Finger): boolean {
  return finger.startsWith('left');
}

/** Propriétés du clavier. */
interface Props {
  /** Graphèmes du texte de la session. */
  text: readonly string[];
  /** Position du prochain graphème à taper. */
  position: number;
}

/** Clavier de la maquette « Mode Copie » : couleur par doigt, touche suivante en bleu. */
export function VirtualKeyboard({ text, position }: Props) {
  const { t } = useTranslation('practice');
  const [layout, setLayout] = useState<KeyboardLayout | null>(null);
  const [presses, setPresses] = useState<Map<string, Press[]>>(new Map());

  // Disposition du profil et touches de chaque caractère du texte, une fois par texte.
  useEffect(() => {
    let active = true;
    const unique = [...new Set(text)];
    Promise.all([unwrap(commands.keyboardLayout()), unwrap(commands.keyboardKeys(unique))])
      .then(([l, keys]) => {
        if (!active) return;
        setLayout(l);
        setPresses(new Map(unique.map((g, i) => [g, keys[i] ?? []])));
      })
      .catch(() => undefined);
    return () => {
      active = false;
    };
  }, [text]);

  // Touches de la frappe suivante, et leurs modificateurs.
  const keysByCode = useMemo(
    () => new Map((layout?.rows ?? []).flat().map((k) => [k.code, k])),
    [layout],
  );
  if (!layout) return null;
  // Rangée la plus longue, en largeurs de touche : elle occupe toute la largeur.
  const widest = Math.max(
    ...layout.rows.map((row) => row.reduce((sum, k) => sum + (k.width ?? 1), 0)),
  );
  const next = text[position];
  const sequence = next === undefined ? [] : (presses.get(next) ?? []);
  const first = sequence[0];
  const firstKey = first ? keysByCode.get(first.code) : undefined;
  const lit = new Set<string>();
  for (const p of sequence) {
    lit.add(p.code);
    const k = keysByCode.get(p.code);
    // Maj de l'autre main, AltGr à droite, verrouillage des majuscules.
    if (p.shift && k) lit.add(isLeft(k.finger) ? 'ShiftRight' : 'ShiftLeft');
    if (p.altgr) lit.add('AltRight');
    if (p.caps) lit.add('CapsLock');
  }

  return (
    <figure className="flex flex-col items-center gap-3">
      <div
        aria-hidden
        className="flex w-full flex-col gap-1.5 rounded-2xl bg-muted p-3"
        style={{ maxWidth: MAX_WIDTH }}
      >
        {layout.rows.map((row, r) => (
          <div key={r} className="flex gap-1.5">
            {row.map((key) => (
              <span
                key={key.code}
                className={cn(
                  'grid h-11 place-items-center rounded-md border-t-4 bg-card font-mono text-sm shadow-xs',
                  FINGER_COLORS[key.finger].split(' ')[0],
                  lit.has(key.code) && 'bg-primary font-bold text-primary-foreground',
                )}
                // Largeur proportionnelle à la rangée la plus longue : le clavier tient
                // dans la page, quelle que soit sa largeur (interstices de 0,375 rem).
                style={{
                  width: `calc(${((key.width ?? 1) / widest) * 100}% - 0.375rem)`,
                }}
              >
                {label(key)}
              </span>
            ))}
          </div>
        ))}
      </div>
      {/* Légende : touche suivante et doigt conseillé, aussi pour les lecteurs d'écran. */}
      <figcaption role="status" className="flex items-center gap-2 text-sm text-muted-foreground">
        {firstKey && (
          <>
            <span
              aria-hidden
              className={cn('size-3 rounded-sm', FINGER_COLORS[firstKey.finger].split(' ')[1])}
            />
            {t('keyboard.next', {
              keys: sequence
                .map((p) => keysByCode.get(p.code))
                .filter((k): k is KeyDef => k !== undefined)
                .map(label)
                .join(t('keyboard.then')),
              finger: t(`keyboard.fingers.${firstKey.finger}`),
            })}
            {first?.shift && ` · ${t('keyboard.withShift')}`}
            {first?.altgr && ` · ${t('keyboard.withAltGr')}`}
            {first?.caps && ` · ${t('keyboard.withCaps')}`}
          </>
        )}
      </figcaption>
    </figure>
  );
}
