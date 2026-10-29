/** Carte « Révision du jour » de l'accueil (F-43). */

import { ArrowRight } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useNavigate } from 'react-router';

import { commands, unwrap, type ReviewOverview } from '@/shared/api';
import { tzOffsetMinutes } from '@/shared/lib/dates';
import { cn } from '@/shared/lib/utils';
import { buttonVariants } from '@/shared/ui/button-variants';

/** Mots montrés en pastilles avant « + N » (maquette : 5). */
const SHOWN_WORDS = 5;
/** Temps estimé par mot à réviser, en secondes, pour l'estimation « ≈ 5 min ». */
const SECONDS_PER_WORD = 25;

/** Classes du bouton ambre de la maquette. */
const amberButton =
  'inline-flex h-11 items-center gap-2 self-start rounded-lg bg-accent px-6 text-sm font-semibold text-accent-foreground outline-none hover:bg-accent/90 focus-visible:ring-2 focus-visible:ring-accent focus-visible:ring-offset-2 focus-visible:ring-offset-sidebar disabled:opacity-60';

/**
 * Révision du jour : nombre de mots dus, les premiers en pastilles, et un bouton qui
 * lance directement l'exercice (« en un clic », F-43) dans le mode Copie.
 */
export function ReviewCard({ overview }: { overview: ReviewOverview | null }) {
  const { t } = useTranslation('home');
  const navigate = useNavigate();
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const due = overview?.due ?? [];

  /** Génère l'exercice des mots du jour, puis l'ouvre dans le mode Copie (ADR 0018). */
  const start = async () => {
    setBusy(true);
    setError(null);
    try {
      const content = await unwrap(
        commands.reviewExercise({ type: 'shuffled' }, tzOffsetMinutes()),
      );
      if (content) navigate('/practice', { state: { review: content } });
    } catch (e) {
      setError(String((e as { message?: string }).message ?? e));
    } finally {
      setBusy(false);
    }
  };

  // Estimation arrondie à la minute, au moins une.
  const minutes = Math.max(1, Math.round((due.length * SECONDS_PER_WORD) / 60));

  return (
    <section className="flex flex-col gap-4 rounded-2xl bg-sidebar p-6 text-sidebar-foreground shadow-xs">
      <p className="flex items-center justify-between text-xs font-semibold tracking-wider uppercase">
        <span className="text-accent">{t('review.eyebrow')}</span>
        {due.length > 0 && (
          <span className="font-normal tracking-normal text-sidebar-muted normal-case">
            {t('review.estimate', { minutes })}
          </span>
        )}
      </p>
      <h2 className="text-3xl leading-tight font-bold">
        {due.length > 0
          ? t('review.due', { count: due.length })
          : overview && overview.tracked > 0
            ? t('review.upToDate')
            : t('review.none')}
      </h2>
      {/* Premiers mots du jour, puis le nombre de mots restants. */}
      {due.length > 0 ? (
        <ul className="flex flex-wrap gap-2">
          {due.slice(0, SHOWN_WORDS).map((w) => (
            <li key={w} className="rounded-md bg-sidebar-accent px-3 py-1.5 font-mono">
              {w}
            </li>
          ))}
          {due.length > SHOWN_WORDS && (
            <li className="px-2 py-1.5 font-mono text-sidebar-muted">
              + {due.length - SHOWN_WORDS}
            </li>
          )}
        </ul>
      ) : (
        <p className="text-sidebar-muted">
          {overview && overview.tracked > 0 ? t('review.upToDateBody') : t('review.noneBody')}
        </p>
      )}
      {error && (
        <p role="alert" className="text-sm text-red-300">
          {t('common:error.generic', { message: error })}
        </p>
      )}
      <div className="mt-auto">
        {due.length > 0 ? (
          <button
            type="button"
            className={amberButton}
            disabled={busy}
            onClick={() => void start()}
          >
            {t('review.start')}
            <ArrowRight aria-hidden className="size-4" />
          </button>
        ) : (
          <Link
            to="/review"
            className={cn(
              buttonVariants({ variant: 'outline' }),
              'border-sidebar-accent bg-transparent text-sidebar-foreground hover:bg-sidebar-accent',
            )}
          >
            {t('review.open')}
          </Link>
        )}
      </div>
    </section>
  );
}
