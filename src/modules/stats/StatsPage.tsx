/** Page Statistiques : progression, clavier, bigrammes et mots difficiles (F-33 à F-35). */

import { Download } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

import {
  commands,
  unwrap,
  type Dashboard_Serialize as Dashboard,
  type KeyReport_Serialize as KeyReport,
  type Period,
} from '@/shared/api';
import { tzOffsetMinutes } from '@/shared/lib/dates';
import { formatHours, formatNumber } from '@/shared/lib/format';
import { cn } from '@/shared/lib/utils';
import { Button } from '@/shared/ui/button';
import { Card, CardTitle } from '@/shared/ui/card';

import { DifficultWords } from './DifficultWords';
import { csvFilename, csvLabels, downloadCsv } from './export';
import { KeyHeatmap } from './KeyHeatmap';
import { SlowBigrams } from './SlowBigrams';
import { SpeedChart } from './SpeedChart';

/** Périodes proposées, dans l'ordre de la maquette. */
const PERIODS: Period[] = ['week', 'month', 'all'];

/** Données de la période choisie. */
interface PeriodData {
  /** Tableau de bord. */
  dashboard: Dashboard;
  /** Carte de chaleur et bigrammes. */
  keys: KeyReport;
}

/** Message lisible d'une erreur de commande. */
function messageOf(e: unknown): string {
  return String((e as { message?: string }).message ?? e);
}

/** Tuile chiffrée de la colonne de droite. */
function Tile({ label, value }: { label: string; value: string }) {
  return (
    <Card className="flex flex-col gap-1">
      <p className="text-sm text-muted-foreground">{label}</p>
      <p className="text-3xl font-bold tracking-tight tabular-nums">{value}</p>
    </Card>
  );
}

/** Statistiques du profil sur 7 jours, 30 jours ou depuis le début. */
export default function StatsPage() {
  const { t, i18n } = useTranslation('stats');
  const lang = i18n.language;
  const [period, setPeriod] = useState<Period>('month');
  const [data, setData] = useState<PeriodData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [exporting, setExporting] = useState(false);

  // Rechargement à chaque changement de période.
  useEffect(() => {
    let active = true;
    const tz = tzOffsetMinutes();
    Promise.all([
      unwrap(commands.statsDashboard(period, tz)),
      unwrap(commands.keyReport(period, tz)),
    ])
      .then(([dashboard, keys]) => {
        if (!active) return;
        setData({ dashboard, keys });
        setError(null);
      })
      .catch((e: unknown) => active && setError(messageOf(e)));
    return () => {
      active = false;
    };
  }, [period]);

  /** Exporte les sessions de la période en CSV (F-35). */
  const exportCsv = async () => {
    setExporting(true);
    try {
      const csv = await unwrap(
        commands.exportSessionsCsv(period, tzOffsetMinutes(), csvLabels(t, lang)),
      );
      downloadCsv(csv, csvFilename());
    } catch (e) {
      setError(messageOf(e));
    } finally {
      setExporting(false);
    }
  };

  const d = data?.dashboard;
  const points = d?.points ?? [];

  return (
    <div className="mx-auto flex max-w-6xl flex-col gap-6">
      {/* En-tête : titre, choix de la période et export. */}
      <header className="flex flex-wrap items-center justify-between gap-4">
        <h1 className="text-4xl font-bold tracking-tight">{t('title')}</h1>
        <div className="flex flex-wrap items-center gap-3">
          <div role="group" aria-label={t('period.label')} className="flex rounded-xl bg-muted p-1">
            {PERIODS.map((p) => (
              <button
                key={p}
                type="button"
                aria-pressed={period === p}
                onClick={() => setPeriod(p)}
                className={cn(
                  'rounded-lg px-4 py-1.5 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring',
                  period === p
                    ? 'bg-card font-semibold shadow-xs'
                    : 'text-muted-foreground hover:text-foreground',
                )}
              >
                {t(`period.${p}`)}
              </button>
            ))}
          </div>
          <Button variant="outline" onClick={() => void exportCsv()} disabled={exporting}>
            <Download aria-hidden className="size-4" />
            {t('export')}
          </Button>
        </div>
      </header>

      {error && (
        <p role="alert" className="text-destructive">
          {t('common:error.generic', { message: error })}
        </p>
      )}

      {!d || !data ? (
        !error && <p>{t('common:loading')}</p>
      ) : (
        <>
          {/* Courbe de vitesse et trois tuiles (F-33). */}
          <div className="grid gap-6 lg:grid-cols-[1fr_20rem]">
            <Card>
              <div className="mb-2 flex flex-wrap items-baseline justify-between gap-2">
                <CardTitle className="mb-0">{t('chart.title')}</CardTitle>
                {points.length > 0 && (
                  <p className="text-sm text-muted-foreground">
                    {t('chart.summary', {
                      from: formatNumber(points[0]?.wpmNet, lang),
                      to: formatNumber(points.at(-1)?.wpmNet, lang),
                      count: points.length,
                    })}
                  </p>
                )}
              </div>
              <SpeedChart points={points} best={d.bestWpmNet} />
            </Card>
            <div className="flex flex-col gap-6">
              <Tile
                label={t('tiles.accuracy')}
                value={
                  d.averageAccuracy == null ? '—' : `${formatNumber(d.averageAccuracy, lang, 1)} %`
                }
              />
              <Tile label={t('tiles.training')} value={formatHours(d.period.trainingMs)} />
              <Tile label={t('tiles.streak')} value={t('days', { count: d.streak.current })} />
            </div>
          </div>

          {/* Carte de chaleur et bigrammes (F-34). */}
          <div className="grid gap-6 lg:grid-cols-[1fr_20rem]">
            <Card>
              <CardTitle>{t('keys.title')}</CardTitle>
              <KeyHeatmap report={data.keys} />
            </Card>
            <Card>
              <CardTitle>{t('bigrams.title')}</CardTitle>
              <SlowBigrams bigrams={data.keys.slowBigrams} />
            </Card>
          </div>
        </>
      )}

      {/* Mots difficiles (F-41, F-45), indépendants de la période. */}
      <Card>
        <DifficultWords />
      </Card>
    </div>
  );
}
