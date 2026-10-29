/** Courbe de vitesse (MPM net) par session, en SVG (F-33). */

import { useId } from 'react';
import { useTranslation } from 'react-i18next';

import type { ProgressPoint } from '@/shared/api';
import { shortDate } from '@/shared/lib/dates';
import { formatNumber } from '@/shared/lib/format';

/** Dimensions de la zone de dessin, en unités du `viewBox`. */
const WIDTH = 640;
const HEIGHT = 260;
/** Marges : à gauche pour les graduations, en bas pour les dates. */
const PAD = { left: 40, right: 16, top: 16, bottom: 32 };

/** Graduations de l'axe vertical : bornes arrondies à la dizaine, pas de 10 ou 20. */
function scale(values: number[]): { lo: number; hi: number; ticks: number[] } {
  const lo = Math.max(0, Math.floor((Math.min(...values) - 5) / 10) * 10);
  const hi = Math.ceil((Math.max(...values) + 5) / 10) * 10;
  // Au-delà de 4 intervalles de 10, on passe à 20 pour rester lisible.
  const step = hi - lo > 40 ? 20 : 10;
  const ticks: number[] = [];
  for (let v = lo; v <= hi; v += step) ticks.push(v);
  return { lo, hi, ticks };
}

/** Propriétés de la courbe. */
interface Props {
  /** Sessions de la période, dans l'ordre chronologique. */
  points: ProgressPoint[];
  /** Record personnel de tous les temps, en MPM net. */
  best: number | null;
}

/** Courbe MPM net, ligne pointillée du record et dates de début et de fin. */
export function SpeedChart({ points, best }: Props) {
  const { t, i18n } = useTranslation('stats');
  const lang = i18n.language;
  const titleId = useId();

  // Sans session, rien à tracer.
  if (points.length === 0) {
    return <p className="py-16 text-center text-muted-foreground">{t('chart.empty')}</p>;
  }

  // Une valeur non numérique (NaN sérialisé en `null`) compte pour 0.
  const values = points.map((p) => p.wpmNet ?? 0);
  const { lo, hi, ticks } = scale(best == null ? values : [...values, best]);
  // Conversion valeur → coordonnées ; une seule session est centrée.
  const innerW = WIDTH - PAD.left - PAD.right;
  const innerH = HEIGHT - PAD.top - PAD.bottom;
  const x = (i: number) =>
    PAD.left + (points.length === 1 ? innerW / 2 : (i / (points.length - 1)) * innerW);
  const y = (v: number) => PAD.top + (1 - (v - lo) / (hi - lo)) * innerH;
  const line = values.map((v, i) => `${x(i).toFixed(1)},${y(v).toFixed(1)}`).join(' ');
  const last = points.length - 1;
  const first = points[0];
  const end = points[last];

  return (
    <svg
      viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
      role="img"
      aria-labelledby={titleId}
      className="h-auto w-full text-muted-foreground"
    >
      <title id={titleId}>
        {t('chart.summary', {
          from: formatNumber(first?.wpmNet, lang),
          to: formatNumber(end?.wpmNet, lang),
          count: points.length,
        })}
      </title>
      {/* Lignes horizontales et graduations. */}
      {ticks.map((v) => (
        <g key={v}>
          <line
            x1={PAD.left}
            x2={WIDTH - PAD.right}
            y1={y(v)}
            y2={y(v)}
            stroke="currentColor"
            strokeOpacity={0.15}
          />
          <text x={PAD.left - 8} y={y(v) + 4} textAnchor="end" fontSize={12} fill="currentColor">
            {v}
          </text>
        </g>
      ))}
      {/* Record personnel : ligne pointillée ambre et son libellé. */}
      {best != null && (
        <g className="text-amber-600 dark:text-accent">
          <line
            x1={PAD.left}
            x2={WIDTH - PAD.right}
            y1={y(best)}
            y2={y(best)}
            stroke="currentColor"
            strokeWidth={2}
            strokeDasharray="6 5"
          />
          <text x={PAD.left + 6} y={y(best) - 6} fontSize={12} fill="currentColor">
            {t('chart.record', { value: formatNumber(best, lang) })}
          </text>
        </g>
      )}
      {/* Courbe et dernier point. */}
      <g className="text-primary">
        <polyline
          points={line}
          fill="none"
          stroke="currentColor"
          strokeWidth={3}
          strokeLinejoin="round"
          strokeLinecap="round"
        />
        {end && <circle cx={x(last)} cy={y(values[last] ?? 0)} r={5} fill="currentColor" />}
      </g>
      {/* Dates de la première et de la dernière session. */}
      {first && (
        <text x={PAD.left} y={HEIGHT - 8} fontSize={12} fill="currentColor">
          {shortDate(first.startedAt, lang)}
        </text>
      )}
      {end && points.length > 1 && (
        <text
          x={WIDTH - PAD.right}
          y={HEIGHT - 8}
          fontSize={12}
          textAnchor="end"
          fill="currentColor"
        >
          {shortDate(end.startedAt, lang)}
        </text>
      )}
    </svg>
  );
}
