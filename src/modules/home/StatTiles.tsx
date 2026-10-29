/** Tuiles chiffrées de l'accueil : vitesse, précision, série, semaine (F-33). */

import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

import type { Dashboard_Serialize as Dashboard } from '@/shared/api';
import { formatDelta, formatHours, formatNumber } from '@/shared/lib/format';
import { cn } from '@/shared/lib/utils';
import { Card } from '@/shared/ui/card';

/** Une tuile : libellé, valeur et unité, puis une ligne de détail. */
function Tile({
  label,
  value,
  unit,
  detail,
}: {
  label: string;
  value: string;
  unit?: string;
  detail: ReactNode;
}) {
  return (
    <Card className="flex flex-col gap-1 p-5">
      <p className="text-sm text-muted-foreground">{label}</p>
      <p className="text-4xl font-bold tracking-tight tabular-nums">
        {value}
        {unit && <span className="ml-2 text-lg font-normal text-muted-foreground">{unit}</span>}
      </p>
      <div className="text-sm">{detail}</div>
    </Card>
  );
}

/** Écart coloré : vert s'il est favorable (ou nul), rouge sinon. */
function Delta({ children, value }: { children: ReactNode; value: number }) {
  return (
    <span
      className={cn(
        'font-semibold',
        value >= 0 ? 'text-emerald-700 dark:text-emerald-400' : 'text-destructive',
      )}
    >
      {children}
    </span>
  );
}

/** Quatre tuiles d'après le tableau de bord de tous les temps. */
export function StatTiles({ dashboard }: { dashboard: Dashboard }) {
  const { t, i18n } = useTranslation('home');
  const lang = i18n.language;
  const { trend, streak, week } = dashboard;

  return (
    <div className="grid gap-6 sm:grid-cols-2 lg:grid-cols-4">
      <Tile
        label={t('tiles.speed')}
        value={trend.wpmNet == null ? '—' : formatNumber(trend.wpmNet, lang)}
        unit={t('tiles.wpm')}
        detail={
          trend.wpmNetDelta == null ? (
            <span className="text-muted-foreground">{t('tiles.lastSessions')}</span>
          ) : (
            <Delta value={trend.wpmNetDelta}>
              {t('tiles.speedDelta', { delta: formatDelta(trend.wpmNetDelta, lang) })}
            </Delta>
          )
        }
      />
      <Tile
        label={t('tiles.accuracy')}
        value={trend.accuracy == null ? '—' : formatNumber(trend.accuracy, lang, 1)}
        unit="%"
        detail={
          trend.accuracyDelta == null ? (
            <span className="text-muted-foreground">{t('tiles.lastSessions')}</span>
          ) : (
            <Delta value={trend.accuracyDelta}>
              {t('tiles.accuracyDelta', { delta: formatDelta(trend.accuracyDelta, lang, 1) })}
            </Delta>
          )
        }
      />
      <Tile
        label={t('tiles.streak')}
        value={formatNumber(streak.current, lang)}
        unit={t('tiles.days', { count: streak.current })}
        detail={
          <span className="text-muted-foreground">{t('tiles.record', { count: streak.best })}</span>
        }
      />
      <Tile
        label={t('tiles.week')}
        value={formatHours(week.trainingMs)}
        detail={
          <span className="text-muted-foreground">
            {t('tiles.sessions', { count: week.sessions })}
          </span>
        }
      />
    </div>
  );
}
