/** Bandeau des statistiques en direct du mode Copie. */

import { useTranslation } from 'react-i18next';

import { cn } from '@/shared/lib/utils';
import { ProgressBar } from '@/shared/ui/progress-bar';

import { formatDuration, formatNumber } from './format';
import type { LiveStats } from './engine/session';

/** Bandeau en direct (F-05) : MPM, précision, temps, erreurs et progression. */
export function LiveBanner({ stats, elapsedMs }: { stats: LiveStats; elapsedMs: number }) {
  const { t, i18n } = useTranslation('practice');
  const lang = i18n.language;
  // Valeur en grand, unité en petit ; les erreurs passent en rouge dès la première.
  const items = [
    { label: t('banner.wpm'), value: formatNumber(stats.wpm, lang) },
    { label: t('banner.accuracy'), value: formatNumber(stats.accuracy, lang, 1) },
    { label: t('banner.time'), value: formatDuration(elapsedMs, true) },
    {
      label: t('banner.errors'),
      value: formatNumber(stats.errors, lang),
      alert: stats.errors > 0,
    },
  ];
  const percent = formatNumber(stats.progress * 100, lang);
  return (
    <div className="flex flex-wrap items-center gap-x-10 gap-y-3 rounded-2xl border border-border bg-card px-7 py-4 shadow-xs">
      {/* Liste de définitions : les lecteurs d'écran annoncent chaque paire libellé / valeur. */}
      <dl aria-label={t('banner.label')} className="flex flex-wrap items-baseline gap-x-10 gap-y-2">
        {items.map(({ label, value, alert }) => (
          // Valeur avant libellé à l'écran, comme sur la maquette (« 52 MPM »).
          <div key={label} className="flex flex-row-reverse items-baseline gap-2">
            <dt className="text-sm text-muted-foreground">{label}</dt>
            <dd
              className={cn(
                'font-mono text-3xl font-semibold tabular-nums',
                alert && 'text-destructive',
              )}
            >
              {value}
            </dd>
          </div>
        ))}
      </dl>
      {/* Progression : barre et pourcentage, à droite. */}
      <div className="flex min-w-48 flex-1 items-center gap-4">
        <ProgressBar value={stats.progress} label={t('banner.progress')} />
        <span className="text-sm font-semibold whitespace-nowrap tabular-nums">{percent} %</span>
      </div>
    </div>
  );
}
