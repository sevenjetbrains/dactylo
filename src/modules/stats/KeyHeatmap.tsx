/** Carte de chaleur du clavier AZERTY : taux d'erreur par touche (F-34). */

import { useTranslation } from 'react-i18next';

import type { KeyReport_Serialize as KeyReport, KeyStat_Serialize as KeyStat } from '@/shared/api';
import { formatNumber } from '@/shared/lib/format';
import { cn } from '@/shared/lib/utils';

/** Rangées affichées : accents et touches mortes, puis les trois rangées de lettres. */
const ROWS = [
  ['é', 'è', 'ç', 'à', 'ù', '^', '¨'],
  ['a', 'z', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p'],
  ['q', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l', 'm'],
  ['w', 'x', 'c', 'v', 'b', 'n'],
];

/** Décalage de chaque rangée, comme sur un vrai clavier. */
const OFFSETS = ['', '', 'ml-6', 'ml-14'];

/** Teintes, de « aucune erreur » à « taux le plus élevé » ; contrastes AA (NF-07). */
const LEVELS = [
  'bg-orange-50 text-slate-900 dark:bg-orange-950/40 dark:text-orange-100',
  'bg-orange-100 text-slate-900',
  'bg-orange-200 text-slate-900',
  'bg-orange-300 text-slate-900',
  'bg-orange-500 text-slate-950',
  'bg-orange-800 text-white',
];

/** Niveau de 0 à 5 : 0 sans erreur, sinon proportionnel au taux le plus élevé. */
function level(stat: KeyStat, maxRate: number): number {
  if (stat.errors === 0 || maxRate === 0) return 0;
  const rate = stat.errors / stat.presses;
  return Math.min(5, Math.max(1, Math.ceil((rate / maxRate) * 5)));
}

/** Carte de chaleur, légende et part des accents dans les erreurs. */
export function KeyHeatmap({ report }: { report: KeyReport }) {
  const { t, i18n } = useTranslation('stats');
  const lang = i18n.language;
  const byKey = new Map(report.keys.map((k) => [k.key, k]));
  // Le taux le plus élevé sert de référence à l'échelle de couleurs.
  const maxRate = Math.max(0, ...report.keys.map((k) => (k.presses ? k.errors / k.presses : 0)));
  const accentShare =
    report.totalErrors > 0 ? (100 * report.accentErrors) / report.totalErrors : null;

  return (
    <div className="flex flex-col gap-4">
      <ul
        aria-label={t('keys.legend')}
        className="flex items-center gap-1 self-end text-xs text-muted-foreground"
      >
        <li>{t('keys.low')}</li>
        {LEVELS.slice(1).map((c) => (
          <li key={c} aria-hidden className={cn('size-4 rounded', c)} />
        ))}
        <li>{t('keys.high')}</li>
      </ul>
      <div className="flex flex-col gap-2 overflow-x-auto pb-1">
        {ROWS.map((row, r) => (
          <div key={row.join('')} className={cn('flex gap-2', OFFSETS[r])}>
            {row.map((key) => {
              const stat = byKey.get(key);
              // Touche jamais tapée : grisée, sans chiffre.
              const label = stat
                ? t('keys.detail', {
                    key,
                    errors: stat.errors,
                    presses: stat.presses,
                    rate: formatNumber((100 * stat.errors) / stat.presses, lang),
                    ms: stat.avgMs == null ? '—' : formatNumber(stat.avgMs, lang),
                  })
                : t('keys.unused', { key });
              return (
                <span
                  key={key}
                  title={label}
                  aria-label={label}
                  role="img"
                  className={cn(
                    'grid size-11 place-items-center rounded-lg font-mono text-base font-semibold uppercase',
                    stat ? LEVELS[level(stat, maxRate)] : 'bg-muted text-muted-foreground',
                  )}
                >
                  {/* Les lettres accentuées restent en minuscules, comme sur les touches. */}
                  <span className={r === 0 ? 'normal-case' : undefined}>{key}</span>
                </span>
              );
            })}
          </div>
        ))}
      </div>
      {accentShare != null && (
        <p className="text-sm text-muted-foreground">
          {t('keys.accentShare', { share: formatNumber(accentShare, lang) })}
        </p>
      )}
    </div>
  );
}
