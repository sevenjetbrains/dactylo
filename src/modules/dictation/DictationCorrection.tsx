/** Correction d'une dictée (F-16) : chiffres, texte corrigé, fautes par type,
 * mots ajoutés aux révisions. */

import { Check } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

import {
  commands,
  unwrap,
  type AlignedToken_Serialize as AlignedToken,
  type DictationSummary_Serialize as Summary,
} from '@/shared/api';
import { formatDelta, formatDuration, formatNumber } from '@/shared/lib/format';
import { cn } from '@/shared/lib/utils';
import { Button } from '@/shared/ui/button';
import { Card, CardTitle } from '@/shared/ui/card';

/** Jetons montrés avant « Afficher tout le texte ». */
const PREVIEW_TOKENS = 80;

/** Propriétés de l'écran de correction. */
interface Props {
  /** Correction renvoyée par le backend. */
  summary: Summary;
  /** Refaire la même dictée. */
  onRedo: () => void;
  /** S'entraîner sur les mots fautifs (exercice généré, puis Copie). */
  onDrill: (words: string[]) => void;
}

/** Tuile chiffrée. */
function Tile({
  label,
  value,
  detail,
  tone,
}: {
  label: string;
  value: string;
  detail: string;
  tone?: 'bad' | 'good';
}) {
  return (
    <Card className="flex flex-col gap-1 p-5">
      <p className="text-sm text-muted-foreground">{label}</p>
      <p
        className={cn(
          'text-4xl font-bold tracking-tight tabular-nums',
          tone === 'bad' && 'text-destructive',
        )}
      >
        {value}
      </p>
      <p
        className={cn(
          'text-sm',
          tone === 'good'
            ? 'font-semibold text-emerald-700 dark:text-emerald-400'
            : 'text-muted-foreground',
        )}
      >
        {detail}
      </p>
    </Card>
  );
}

/** Un jeton du texte corrigé : juste, ou tapé barré puis attendu en vert. */
function Token({ token }: { token: AlignedToken }) {
  const space = token.spaceBefore ? ' ' : '';
  if (!token.error) return <>{`${space}${token.expected ?? ''}`}</>;
  return (
    <>
      {space}
      {token.typed && (
        <del className="text-[0.85em] text-destructive decoration-destructive">{token.typed}</del>
      )}
      {token.typed && token.expected && ' '}
      {token.expected && (
        <ins className="rounded bg-emerald-100 px-1 font-semibold text-emerald-800 no-underline shadow-[inset_0_-2px_0] shadow-emerald-600 dark:bg-emerald-900/40 dark:text-emerald-100">
          {token.expected}
        </ins>
      )}
    </>
  );
}

/** Écran de correction de la maquette « Correction de la dictée ». */
export function DictationCorrection({ summary, onRedo, onDrill }: Props) {
  const { t, i18n } = useTranslation('dictation');
  const lang = i18n.language;
  const { correction, stats, average } = summary;
  const [showAll, setShowAll] = useState(false);
  const [removed, setRemoved] = useState<string[]>([]);

  // Mots fautifs suivis en révision : mots attendus d'un seul tenant.
  const mistakes = correction.tokens.filter(
    (tk) => tk.word && tk.error && tk.expected && !tk.expected.includes(' '),
  );
  const drillWords = [...new Set(mistakes.map((tk) => tk.expected ?? ''))].filter(Boolean);
  const accuracy = stats.accuracy ?? 0;
  // Message selon la précision.
  const praise = accuracy >= 97 ? 'excellent' : accuracy >= 85 ? 'good' : 'keepGoing';
  const tokens = showAll ? correction.tokens : correction.tokens.slice(0, PREVIEW_TOKENS);
  const maxCount = Math.max(1, ...correction.byType.map((b) => b.count));

  /**
   * Retire un mot de la révision (F-45). Les mots sont enregistrés en tâche de fond
   * après la correction : un échec (mot pas encore enregistré) est retenté une fois.
   */
  const remove = (word: string) => {
    const attempt = () => unwrap(commands.removeWord(word));
    attempt()
      .catch(() => new Promise((r) => setTimeout(r, 800)).then(attempt))
      .then(() => setRemoved((r) => [...r, word]))
      .catch(() => undefined);
  };

  return (
    <div className="flex flex-col gap-6">
      {/* En-tête : bilan et actions. */}
      <header className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="text-xs font-semibold tracking-wider text-emerald-700 uppercase dark:text-emerald-400">
            {t('correction.eyebrow')}
          </p>
          <h1 className="mt-1 text-4xl font-bold tracking-tight">
            {t('correction.title', { count: stats.faults, words: correction.expectedWords })}{' '}
            {t(`correction.praise.${praise}`)}
          </h1>
        </div>
        <div className="flex flex-wrap gap-3">
          <Button variant="outline" onClick={onRedo}>
            {t('correction.redo')}
          </Button>
          {drillWords.length > 0 && (
            <Button onClick={() => onDrill(drillWords)}>
              {t('correction.drill', { count: drillWords.length })}
            </Button>
          )}
        </div>
      </header>

      {/* Tuiles : précision, fautes, vitesse, durée. */}
      <div className="grid gap-6 sm:grid-cols-2 lg:grid-cols-4">
        <Tile
          label={t('correction.accuracy')}
          value={`${formatNumber(accuracy, lang, 1)} %`}
          detail={
            average?.accuracy != null
              ? t('correction.vsAverage', {
                  delta: formatDelta(accuracy - average.accuracy, lang, 1),
                })
              : t('correction.firstDictation')
          }
          tone={average?.accuracy != null && accuracy >= average.accuracy ? 'good' : undefined}
        />
        <Tile
          label={t('correction.faults')}
          value={formatNumber(stats.faults, lang)}
          detail={
            average?.faults != null
              ? t('correction.average', { value: formatNumber(average.faults, lang) })
              : '—'
          }
          tone={stats.faults > 0 ? 'bad' : undefined}
        />
        <Tile
          label={t('correction.speed')}
          value={t('correction.wpm', { value: formatNumber(stats.wpm, lang) })}
          detail={
            average?.wpm != null
              ? t('correction.average', { value: formatNumber(average.wpm, lang) })
              : '—'
          }
        />
        <Tile
          label={t('correction.duration')}
          value={formatDuration(stats.durationMs)}
          detail={t('correction.repeats', { count: stats.repeats })}
        />
      </div>

      <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_22rem]">
        {/* Texte corrigé : tapé barré en rouge, attendu en vert. */}
        <Card>
          <div className="mb-4 flex flex-wrap items-baseline justify-between gap-2">
            <CardTitle className="mb-0">{t('correction.text')}</CardTitle>
            <p className="flex gap-4 text-sm">
              <del className="text-destructive">{t('correction.typedLegend')}</del>
              <ins className="font-semibold text-emerald-700 no-underline dark:text-emerald-400">
                {t('correction.expectedLegend')}
              </ins>
            </p>
          </div>
          <p className="font-mono text-lg leading-loose whitespace-pre-wrap">
            {tokens.map((tk, i) => (
              <Token key={i} token={tk} />
            ))}
          </p>
          {correction.tokens.length > PREVIEW_TOKENS && (
            <button
              type="button"
              className="mt-4 text-sm font-semibold text-primary hover:underline"
              onClick={() => setShowAll((s) => !s)}
            >
              {showAll
                ? t('correction.showLess')
                : t('correction.showAll', { count: correction.expectedWords })}
            </button>
          )}
        </Card>

        {/* Fautes par type, en barres. */}
        <Card>
          <CardTitle>{t('correction.byType')}</CardTitle>
          <ul className="flex flex-col gap-3">
            {correction.byType.map((b) => (
              <li
                key={b.errorType}
                className={cn(
                  'grid grid-cols-[1fr_6rem_1.5rem] items-center gap-3 text-sm',
                  b.count === 0 && 'text-muted-foreground',
                )}
              >
                <span>{t(`errorType.${b.errorType}`)}</span>
                <span aria-hidden className="h-2 overflow-hidden rounded-full bg-muted">
                  <span
                    className="block h-full rounded-full bg-primary"
                    style={{ width: `${Math.round((100 * b.count) / maxCount)}%` }}
                  />
                </span>
                <span className="text-right font-mono tabular-nums">{b.count}</span>
              </li>
            ))}
          </ul>
        </Card>
      </div>

      {/* Mots ajoutés aux révisions (F-40), retirables (F-45). */}
      {mistakes.length > 0 && (
        <Card>
          <div className="mb-4 flex flex-wrap items-baseline justify-between gap-2">
            <CardTitle className="mb-0">{t('correction.addedWords')}</CardTitle>
            <p className="flex items-center gap-1 text-sm font-semibold text-emerald-700 dark:text-emerald-400">
              <Check aria-hidden className="size-4" />
              {t('correction.reviewTomorrow')}
            </p>
          </div>
          <table className="w-full text-left text-sm">
            <thead className="text-xs tracking-wider text-muted-foreground uppercase">
              <tr className="border-b border-border">
                <th className="py-3 pr-4 font-semibold">{t('correction.columns.expected')}</th>
                <th className="py-3 pr-4 font-semibold">{t('correction.columns.typed')}</th>
                <th className="py-3 pr-4 font-semibold">{t('correction.columns.type')}</th>
                <th className="py-3 text-right font-semibold">{t('correction.columns.action')}</th>
              </tr>
            </thead>
            <tbody>
              {mistakes.map((tk, i) => {
                const word = tk.expected ?? '';
                const gone = removed.includes(word);
                return (
                  <tr key={`${word}-${i}`} className="border-b border-border last:border-0">
                    <td className="py-3 pr-4 font-mono text-base font-semibold">{word}</td>
                    <td className="py-3 pr-4 font-mono text-base text-destructive line-through">
                      {tk.typed ?? '—'}
                    </td>
                    <td className="py-3 pr-4">
                      <span className="rounded-md bg-amber-100 px-2 py-0.5 text-xs font-medium whitespace-nowrap text-amber-900 dark:bg-amber-900/40 dark:text-amber-100">
                        {tk.error && t(`errorType.${tk.error}`)}
                      </span>
                    </td>
                    <td className="py-3 text-right">
                      {gone ? (
                        <span className="text-muted-foreground">{t('correction.removed')}</span>
                      ) : (
                        <button
                          type="button"
                          className="text-muted-foreground hover:underline"
                          aria-label={t('correction.removeWord', { word })}
                          onClick={() => remove(word)}
                        >
                          {t('correction.remove')}
                        </button>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </Card>
      )}
    </div>
  );
}
