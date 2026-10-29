/** Liste des bigrammes les plus lents (F-34). */

import { useTranslation } from 'react-i18next';

import type { BigramStat } from '@/shared/api';
import { formatNumber } from '@/shared/lib/format';

/** Bigrammes, du plus lent au moins lent, avec une barre proportionnelle au temps. */
export function SlowBigrams({ bigrams }: { bigrams: BigramStat[] }) {
  const { t, i18n } = useTranslation('stats');
  // Pas assez de frappes pour une moyenne fiable.
  if (bigrams.length === 0) {
    return <p className="text-sm text-muted-foreground">{t('bigrams.empty')}</p>;
  }
  // Le plus lent remplit la barre ; les autres en proportion.
  const max = Math.max(...bigrams.map((b) => b.avgMs ?? 0));
  return (
    <ul className="flex flex-col gap-3">
      {bigrams.map((b) => (
        <li key={b.bigram} className="grid grid-cols-[3rem_1fr_4.5rem] items-center gap-3">
          <span className="font-mono text-lg font-semibold">{b.bigram}</span>
          <span aria-hidden className="h-1.5 overflow-hidden rounded-full bg-muted">
            <span
              className="block h-full rounded-full bg-orange-600"
              style={{ width: `${max > 0 ? Math.round((100 * (b.avgMs ?? 0)) / max) : 0}%` }}
            />
          </span>
          <span className="text-right font-mono text-sm text-muted-foreground tabular-nums">
            {t('bigrams.ms', { value: formatNumber(b.avgMs, i18n.language) })}
          </span>
        </li>
      ))}
    </ul>
  );
}
