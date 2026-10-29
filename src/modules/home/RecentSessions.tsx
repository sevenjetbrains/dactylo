/** Tableau « Sessions récentes » de l'accueil. */

import { useTranslation } from 'react-i18next';
import { Link } from 'react-router';

import type { RecentSession_Serialize as RecentSession, TrainingMode } from '@/shared/api';
import { sessionDate } from '@/shared/lib/dates';
import { formatNumber } from '@/shared/lib/format';
import { cn } from '@/shared/lib/utils';
import { Card } from '@/shared/ui/card';

/** Couleur de l'étiquette de chaque mode, comme sur la maquette. */
const MODE_BADGE: Record<TrainingMode, string> = {
  copy: 'bg-blue-100 text-blue-800 dark:bg-blue-900/40 dark:text-blue-100',
  dictation: 'bg-amber-100 text-amber-900 dark:bg-amber-900/40 dark:text-amber-100',
  review: 'bg-emerald-100 text-emerald-800 dark:bg-emerald-900/40 dark:text-emerald-100',
};

/** Dernières sessions terminées : date, texte, mode, MPM et précision. */
export function RecentSessions({ sessions }: { sessions: RecentSession[] }) {
  const { t, i18n } = useTranslation('home');
  const lang = i18n.language;

  return (
    <Card>
      <div className="mb-4 flex items-baseline justify-between">
        <h2 className="text-lg font-semibold">{t('recent.title')}</h2>
        <Link to="/stats" className="text-sm font-semibold text-primary hover:underline">
          {t('recent.all')}
        </Link>
      </div>
      {sessions.length === 0 ? (
        <p className="text-muted-foreground">{t('recent.empty')}</p>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-left text-sm">
            <thead className="text-xs tracking-wider text-muted-foreground uppercase">
              <tr className="border-b border-border">
                <th className="py-3 pr-4 font-semibold">{t('recent.date')}</th>
                <th className="py-3 pr-4 font-semibold">{t('recent.text')}</th>
                <th className="py-3 pr-4 font-semibold">{t('recent.mode')}</th>
                <th className="py-3 pr-4 text-right font-semibold">{t('recent.wpm')}</th>
                <th className="py-3 text-right font-semibold">{t('recent.accuracy')}</th>
              </tr>
            </thead>
            <tbody>
              {sessions.map((s) => (
                <tr key={s.sessionId} className="border-b border-border last:border-0">
                  <td className="py-3 pr-4 text-muted-foreground">
                    {sessionDate(s.startedAt, lang)}
                  </td>
                  {/* Une révision n'a pas de texte stocké : son mode lui sert de titre. */}
                  <td className="py-3 pr-4 font-medium">{s.title ?? t('recent.reviewTitle')}</td>
                  <td className="py-3 pr-4">
                    <span
                      className={cn(
                        'rounded-md px-2 py-0.5 text-xs font-medium',
                        MODE_BADGE[s.mode],
                      )}
                    >
                      {t(`recent.modes.${s.mode}`)}
                    </span>
                  </td>
                  <td className="py-3 pr-4 text-right font-mono tabular-nums">
                    {formatNumber(s.wpmNet, lang)}
                  </td>
                  <td className="py-3 text-right font-mono tabular-nums">
                    {formatNumber(s.accuracy, lang, 1)} %
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </Card>
  );
}
