import { useTranslation } from 'react-i18next';
import { Link } from 'react-router';

import type { SessionSummary_Serialize as Summary } from '@/shared/api';
import { Button } from '@/shared/ui/button';
import { Card, CardTitle } from '@/shared/ui/card';
import { cn } from '@/shared/lib/utils';

import { formatDuration, formatNumber } from './format';

interface Props {
  summary: Summary;
  onRestart: () => void;
  onNext: () => void;
}

/** Écran de fin de session (F-32) : chiffres, mots fautifs, comparaison à la moyenne. */
export function SessionSummary({ summary, onRestart, onNext }: Props) {
  const { t, i18n } = useTranslation('practice');
  const { stats, average, errors } = summary;
  const lang = i18n.language;

  const delta = (value: number | null, mean: number | null | undefined, unit = '') => {
    if (average == null || mean == null) return null;
    const diff = (value ?? 0) - mean;
    const sign = diff > 0 ? '+' : diff < 0 ? '−' : '±';
    return (
      <span
        className={cn(
          'text-xs',
          diff >= 0 ? 'text-emerald-700 dark:text-emerald-400' : 'text-destructive',
        )}
      >
        {t('summary.vsAverage', { delta: `${sign}${formatNumber(Math.abs(diff), lang)}${unit}` })}
      </span>
    );
  };

  const tiles = [
    {
      label: t('summary.wpmNet'),
      value: formatNumber(stats.wpmNet, lang),
      delta: delta(stats.wpmNet, average?.wpmNet),
    },
    {
      label: t('summary.accuracy'),
      value: `${formatNumber(stats.accuracy, lang)} %`,
      delta: delta(stats.accuracy, average?.accuracy, ' %'),
    },
    {
      label: t('summary.cpm'),
      value: formatNumber(stats.cpm, lang),
      delta: delta(stats.cpm, average?.cpm),
    },
    { label: t('summary.wpmRaw'), value: formatNumber(stats.wpmRaw, lang), delta: null },
    { label: t('summary.duration'), value: formatDuration(stats.durationMs), delta: null },
    { label: t('summary.errors'), value: formatNumber(stats.errors, lang), delta: null },
  ];
  const words = errors.filter((e) => e.word);

  return (
    <div className="flex flex-col gap-6">
      <h2 className="text-xl font-semibold">{t('summary.title')}</h2>
      <dl className="grid grid-cols-2 gap-4 sm:grid-cols-3">
        {tiles.map((tile) => (
          <div key={tile.label} className="rounded-lg border border-border bg-card p-4">
            <dt className="text-sm text-muted-foreground">{tile.label}</dt>
            <dd className="font-mono text-2xl font-semibold tabular-nums">{tile.value}</dd>
            {tile.delta}
          </div>
        ))}
      </dl>
      <p className="text-sm text-muted-foreground">
        {average
          ? t('summary.averageOver', { count: average.sessions })
          : t('summary.firstSession')}
      </p>
      <Card>
        <CardTitle>{t('summary.mistyped', { count: words.length })}</CardTitle>
        {words.length === 0 ? (
          <p className="text-muted-foreground">{t('summary.noMistake')}</p>
        ) : (
          <ul className="flex flex-col gap-1">
            {words.map((e) => (
              <li key={e.position} className="flex flex-wrap items-baseline gap-2">
                <span className="font-mono font-semibold">{e.expected}</span>
                <span className="font-mono text-destructive line-through">{e.typed}</span>
                <span className="text-xs text-muted-foreground">
                  {t(`errorType.${e.errorType}`)}
                </span>
              </li>
            ))}
          </ul>
        )}
      </Card>
      <div className="flex flex-wrap gap-3">
        <Button onClick={onRestart}>{t('summary.restart')}</Button>
        <Button variant="outline" onClick={onNext}>
          {t('summary.next')}
        </Button>
        <Link to="/library" className="inline-flex h-9 items-center px-2 text-sm underline">
          {t('summary.library')}
        </Link>
      </div>
    </div>
  );
}
