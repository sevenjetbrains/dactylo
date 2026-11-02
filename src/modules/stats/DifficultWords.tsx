/** Tableau « Mots difficiles » de la page Statistiques (F-41, F-45). */

import { useCallback, useEffect, useId, useState, type FormEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router';

import {
  commands,
  unwrap,
  type DifficultWord_Serialize as DifficultWord,
  type ReviewOverview,
} from '@/shared/api';
import { dayDiff, relativeDay, tzOffsetMinutes } from '@/shared/lib/dates';
import { usePagination } from '@/shared/lib/use-pagination';
import { useDomainEvent } from '@/shared/lib/use-domain-event';
import { cn } from '@/shared/lib/utils';
import { Button } from '@/shared/ui/button';
import { buttonVariants } from '@/shared/ui/button-variants';
import { Pagination } from '@/shared/ui/pagination';

/** Réussites de suite pour la maîtrise (F-42), une pastille chacune. */
const MASTERY_STREAK = 3;
/** Nombre de mots par page du tableau. */
const PAGE_SIZE = 15;

/** Message lisible d'une erreur de commande. */
function messageOf(e: unknown): string {
  return String((e as { message?: string }).message ?? e);
}

/** Mots en révision, avec ajout, maîtrise et retrait à la main. */
export function DifficultWords() {
  const { t, i18n } = useTranslation('stats');
  const lang = i18n.language;
  const id = useId();
  const [words, setWords] = useState<DifficultWord[] | null>(null);
  const [overview, setOverview] = useState<ReviewOverview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);
  const [draft, setDraft] = useState('');
  // Chaque incrément recharge la liste.
  const [version, setVersion] = useState(0);
  const reload = useCallback(() => setVersion((v) => v + 1), []);
  useDomainEvent('wordsUpdated', reload);
  const { page, pageCount, pageItems, setPage } = usePagination(words ?? [], PAGE_SIZE);

  useEffect(() => {
    // `active` ignore une réponse arrivée après le démontage ou un rechargement.
    let active = true;
    Promise.all([
      unwrap(commands.difficultWords()),
      unwrap(commands.reviewOverview(tzOffsetMinutes())),
    ])
      .then(([w, o]) => {
        if (!active) return;
        setWords(w);
        setOverview(o);
      })
      .catch((e: unknown) => active && setError(messageOf(e)));
    return () => {
      active = false;
    };
  }, [version]);

  /** Lance une action sur un mot puis recharge la liste. */
  const act = (call: Promise<unknown>) => {
    setError(null);
    call.then(reload).catch((e: unknown) => setError(messageOf(e)));
  };

  /** Ajoute le mot saisi (F-45). */
  const submit = (e: FormEvent) => {
    e.preventDefault();
    if (!draft.trim()) return;
    act(
      unwrap(commands.addWord(draft)).then(() => {
        setDraft('');
        setAdding(false);
      }),
    );
  };

  const due = overview?.due.length ?? 0;

  return (
    <div className="flex flex-col gap-4">
      {/* En-tête : titre, compteurs et actions. */}
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h2 className="text-lg font-semibold">{t('words.title')}</h2>
          {overview && (
            <p className="text-sm text-muted-foreground">
              {t('words.counts', {
                due,
                tracked: overview.tracked,
                mastered: overview.mastered,
              })}
            </p>
          )}
        </div>
        <div className="flex flex-wrap gap-3">
          <Button variant="outline" onClick={() => setAdding((a) => !a)} aria-expanded={adding}>
            {t('words.add')}
          </Button>
          {due > 0 && (
            <Link to="/review" className={buttonVariants()}>
              {t('words.review', { count: due })}
            </Link>
          )}
        </div>
      </div>

      {/* Saisie d'un mot à ajouter. */}
      {adding && (
        <form onSubmit={submit} className="flex flex-wrap items-end gap-3">
          <label htmlFor={`${id}-word`} className="flex flex-col gap-1 text-sm">
            {t('words.addLabel')}
            <input
              id={`${id}-word`}
              autoFocus
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              className="h-9 rounded-md border border-border bg-background px-2 font-mono outline-none focus-visible:ring-2 focus-visible:ring-ring"
            />
          </label>
          <Button type="submit" size="sm" className="h-9">
            {t('words.addSubmit')}
          </Button>
        </form>
      )}

      {error && (
        <p role="alert" className="text-destructive">
          {t('common:error.generic', { message: error })}
        </p>
      )}

      {words === null ? (
        <p>{t('common:loading')}</p>
      ) : words.length === 0 ? (
        <p className="text-muted-foreground">{t('words.empty')}</p>
      ) : (
        <>
          <div className="overflow-x-auto">
            <table className="w-full text-left text-sm">
              <thead className="text-xs tracking-wider text-muted-foreground uppercase">
                <tr className="border-b border-border">
                  <th className="py-3 pr-4 font-semibold">{t('words.columns.word')}</th>
                  <th className="py-3 pr-4 font-semibold">{t('words.columns.typed')}</th>
                  <th className="py-3 pr-4 font-semibold">{t('words.columns.error')}</th>
                  <th className="py-3 pr-4 text-right font-semibold">{t('words.columns.ratio')}</th>
                  <th className="py-3 pr-4 font-semibold">{t('words.columns.mastery')}</th>
                  <th className="py-3 pr-4 font-semibold">{t('words.columns.review')}</th>
                  <th className="py-3">
                    <span className="sr-only">{t('words.columns.actions')}</span>
                  </th>
                </tr>
              </thead>
              <tbody>
                {pageItems.map((w) => {
                  // Révision due aujourd'hui (ou en retard) : en ambre, comme sur la maquette.
                  const overdue = w.nextReviewAt != null && dayDiff(w.nextReviewAt) <= 0;
                  return (
                    <tr key={w.word} className="border-b border-border last:border-0">
                      <td className="py-3 pr-4 font-mono text-base font-semibold">{w.word}</td>
                      <td className="py-3 pr-4 font-mono text-base text-destructive">
                        {w.typedForms.join(', ') || '—'}
                      </td>
                      <td className="py-3 pr-4">
                        {w.dominantError ? (
                          <span className="rounded-md bg-amber-100 px-2 py-0.5 text-xs font-medium whitespace-nowrap text-amber-900 dark:bg-amber-900/40 dark:text-amber-100">
                            {t(`errorType.${w.dominantError}`)}
                          </span>
                        ) : w.encounters > 0 ? (
                          // Suivi sans faute : il est entré par sa lenteur (F-40).
                          <span className="text-muted-foreground">{t('words.slow')}</span>
                        ) : (
                          // Ajouté à la main, pas encore tapé.
                          <span className="text-muted-foreground">—</span>
                        )}
                      </td>
                      <td className="py-3 pr-4 text-right font-mono tabular-nums">
                        {w.errors} / {w.encounters}
                      </td>
                      <td className="py-3 pr-4">
                        {/* Une pastille verte par réussite de suite, sur 3. */}
                        <span
                          role="img"
                          aria-label={t('words.streak', { count: w.streak, total: MASTERY_STREAK })}
                          className="flex gap-1"
                        >
                          {Array.from({ length: MASTERY_STREAK }, (_, i) => (
                            <span
                              key={i}
                              className={cn(
                                'size-2.5 rounded-full',
                                i < w.streak ? 'bg-success' : 'bg-border',
                              )}
                            />
                          ))}
                        </span>
                      </td>
                      <td
                        className={cn(
                          'py-3 pr-4',
                          overdue && 'font-semibold text-amber-800 dark:text-accent',
                        )}
                      >
                        {w.nextReviewAt == null
                          ? '—'
                          : overdue
                            ? t('words.today')
                            : relativeDay(w.nextReviewAt, lang)}
                      </td>
                      <td className="py-3 text-right whitespace-nowrap">
                        <button
                          type="button"
                          className="font-semibold text-primary hover:underline"
                          onClick={() => act(unwrap(commands.markWordMastered(w.word)))}
                          aria-label={t('words.markMasteredWord', { word: w.word })}
                        >
                          {t('words.markMastered')}
                        </button>
                        <button
                          type="button"
                          className="ml-4 text-muted-foreground hover:underline"
                          onClick={() => act(unwrap(commands.removeWord(w.word)))}
                          aria-label={t('words.removeWord', { word: w.word })}
                        >
                          {t('words.remove')}
                        </button>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
          <Pagination page={page} pageCount={pageCount} onPage={setPage} />
        </>
      )}
    </div>
  );
}
