/** Page du module Révision : mots du jour et choix de l'exercice (F-43, F-44). */

import { Repeat, Shuffle, TextQuote, type LucideIcon } from 'lucide-react';
import { useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useNavigate } from 'react-router';

import { commands, unwrap, type ReviewExerciseKind } from '@/shared/api';
import { tzOffsetMinutes } from '@/shared/lib/dates';
import { Button } from '@/shared/ui/button';
import { Card, CardTitle, Eyebrow } from '@/shared/ui/card';

import { useReviewOverview } from './use-review-overview';

/** Mots affichés en pastilles avant « + N ». */
const SHOWN_WORDS = 12;
/** Répétitions proposées pour l'exercice « mots répétés ». */
const REPEATS = [2, 3, 4, 5];
/** Classes d'une liste déroulante. */
const selectClass =
  'h-9 rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring';

/** Une forme d'exercice proposée. */
interface ExerciseOption {
  /** Clé de traduction et identifiant. */
  id: 'shuffled' | 'repeated' | 'sentences';
  /** Icône de la carte. */
  icon: LucideIcon;
}

/** Les trois formes d'exercice ; la dictée de ces mots viendra en Phase 4. */
const OPTIONS: ExerciseOption[] = [
  { id: 'shuffled', icon: Shuffle },
  { id: 'repeated', icon: Repeat },
  { id: 'sentences', icon: TextQuote },
];

/** Révision : mots à revoir aujourd'hui, puis exercice généré tapé dans le mode Copie. */
export default function ReviewPage() {
  const { t } = useTranslation('review');
  const navigate = useNavigate();
  const id = useId();
  const { overview, error } = useReviewOverview();
  const [times, setTimes] = useState(3);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  /** Génère l'exercice puis l'ouvre dans le mode Copie (ADR 0018). */
  const launch = async (option: ExerciseOption['id']) => {
    const kind: ReviewExerciseKind =
      option === 'repeated' ? { type: 'repeated', times } : { type: option };
    setBusy(true);
    setMessage(null);
    try {
      const content = await unwrap(commands.reviewExercise(kind, tzOffsetMinutes()));
      // Rien à taper : aucune phrase trouvée, ou plus aucun mot suivi.
      if (content === '') {
        setMessage(option === 'sentences' ? t('noSentence') : t('nothingToReview'));
        return;
      }
      navigate('/practice', { state: { review: content } });
    } catch (e) {
      setMessage(t('common:error.generic', { message: (e as { message?: string }).message ?? e }));
    } finally {
      setBusy(false);
    }
  };

  // Erreur ou chargement avant tout le reste.
  if (error) {
    return (
      <p role="alert" className="text-destructive">
        {t('common:error.generic', { message: error })}
      </p>
    );
  }
  if (!overview) return <p>{t('common:loading')}</p>;

  const due = overview.due;
  // Mots du jour s'il y en a ; sinon on peut s'avancer sur les mots suivis.
  const hasWords = overview.tracked > 0;

  return (
    <div className="mx-auto flex max-w-6xl flex-col gap-6">
      <header>
        <Eyebrow className="text-amber-700 dark:text-accent">{t('menu')}</Eyebrow>
        <h1 className="mt-1 text-3xl font-bold tracking-tight">
          {due.length > 0 ? t('dueTitle', { count: due.length }) : t('nothingDueTitle')}
        </h1>
        <p className="mt-1 text-muted-foreground">
          {t('counts', { tracked: overview.tracked, mastered: overview.mastered })}
        </p>
      </header>

      {/* Mots du jour, en pastilles monospace comme sur la maquette. */}
      {due.length > 0 && (
        <ul aria-label={t('dueList')} className="flex flex-wrap gap-2">
          {due.slice(0, SHOWN_WORDS).map((w) => (
            <li key={w} className="rounded-md bg-muted px-3 py-1.5 font-mono">
              {w}
            </li>
          ))}
          {due.length > SHOWN_WORDS && (
            <li className="px-2 py-1.5 font-mono text-muted-foreground">
              + {due.length - SHOWN_WORDS}
            </li>
          )}
        </ul>
      )}

      {/* Aucun mot suivi : explication de l'entrée dans la liste (F-40). */}
      {!hasWords ? (
        <Card>
          <CardTitle>{t('empty.title')}</CardTitle>
          <p className="mb-4 text-muted-foreground">{t('empty.body')}</p>
          <Link to="/practice" className="font-semibold text-primary underline">
            {t('empty.action')}
          </Link>
        </Card>
      ) : (
        <>
          {/* Sans mot dû, l'exercice porte sur les prochains mots à réviser. */}
          {due.length === 0 && <p className="text-muted-foreground">{t('aheadHint')}</p>}
          <div className="grid gap-6 lg:grid-cols-3">
            {OPTIONS.map(({ id: option, icon: Icon }) => (
              <Card key={option} className="flex flex-col gap-3">
                <Eyebrow>
                  <Icon aria-hidden className="size-4" />
                  {t(`exercises.${option}.eyebrow`)}
                </Eyebrow>
                <h2 className="text-lg font-bold">{t(`exercises.${option}.title`)}</h2>
                <p className="text-sm text-muted-foreground">{t(`exercises.${option}.body`)}</p>
                {/* Nombre de répétitions, pour la seule forme « mots répétés ». */}
                {option === 'repeated' && (
                  <label htmlFor={`${id}-times`} className="flex items-center gap-2 text-sm">
                    {t('times')}
                    <select
                      id={`${id}-times`}
                      className={selectClass}
                      value={times}
                      onChange={(e) => setTimes(Number(e.target.value))}
                    >
                      {REPEATS.map((n) => (
                        <option key={n} value={n}>
                          {t('timesValue', { count: n })}
                        </option>
                      ))}
                    </select>
                  </label>
                )}
                <Button
                  className="mt-auto self-start"
                  disabled={busy}
                  onClick={() => void launch(option)}
                >
                  {t('start')}
                </Button>
              </Card>
            ))}
          </div>
        </>
      )}

      {message && (
        <p role="status" className="text-muted-foreground">
          {message}
        </p>
      )}

      <Link to="/stats" className="self-start text-sm font-semibold text-primary underline">
        {t('allWords')}
      </Link>
    </div>
  );
}
