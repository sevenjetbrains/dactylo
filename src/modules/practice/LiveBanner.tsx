/** Bandeau des statistiques en direct du mode Copie. */

import { useTranslation } from 'react-i18next';

import { formatDuration, formatNumber } from './format';
import type { LiveStats } from './engine/session';

/** Bandeau en direct (F-05) : MPM, précision, temps, progression. */
export function LiveBanner({ stats, elapsedMs }: { stats: LiveStats; elapsedMs: number }) {
  const { t, i18n } = useTranslation('practice');
  // Libellé et valeur formatée de chaque indicateur, dans l'ordre d'affichage.
  const items = [
    { label: t('banner.wpm'), value: formatNumber(stats.wpm, i18n.language) },
    { label: t('banner.accuracy'), value: `${formatNumber(stats.accuracy, i18n.language)} %` },
    { label: t('banner.time'), value: formatDuration(elapsedMs) },
    {
      label: t('banner.progress'),
      value: `${formatNumber(stats.progress * 100, i18n.language)} %`,
    },
  ];
  return (
    // Liste de définitions : les lecteurs d'écran annoncent chaque paire libellé / valeur.
    <dl aria-label={t('banner.label')} className="flex flex-wrap gap-x-8 gap-y-2 text-sm">
      {items.map(({ label, value }) => (
        <div key={label} className="flex items-baseline gap-2">
          <dt className="text-muted-foreground">{label}</dt>
          <dd className="font-mono text-lg font-semibold tabular-nums">{value}</dd>
        </div>
      ))}
    </dl>
  );
}
