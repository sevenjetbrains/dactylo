/** Vue d'ensemble de la révision, rafraîchie quand les mots changent. */

import { useCallback, useEffect, useState } from 'react';

import { commands, unwrap, type ReviewOverview } from '@/shared/api';
import { tzOffsetMinutes } from '@/shared/lib/dates';
import { useDomainEvent } from '@/shared/lib/use-domain-event';

/** État renvoyé par `useReviewOverview`. */
export interface ReviewOverviewState {
  /** Vue d'ensemble, `null` pendant le premier chargement. */
  overview: ReviewOverview | null;
  /** Message d'erreur du dernier chargement. */
  error: string | null;
  /** Relance le chargement (après un ajout ou un retrait de mot). */
  refresh: () => void;
}

/**
 * Mots à réviser aujourd'hui, suivis et maîtrisés (F-43). Rechargée à chaque
 * `WordsUpdated` : fin de session, ajout, retrait ou maîtrise d'un mot.
 */
export function useReviewOverview(): ReviewOverviewState {
  const [overview, setOverview] = useState<ReviewOverview | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Chaque incrément relance le chargement.
  const [version, setVersion] = useState(0);
  const refresh = useCallback(() => setVersion((v) => v + 1), []);
  useDomainEvent('wordsUpdated', refresh);

  useEffect(() => {
    // `active` ignore une réponse arrivée après le démontage ou un nouveau chargement.
    let active = true;
    unwrap(commands.reviewOverview(tzOffsetMinutes()))
      .then((o) => {
        if (!active) return;
        setOverview(o);
        setError(null);
      })
      .catch((e: unknown) => {
        if (active) setError(String((e as { message?: string }).message ?? e));
      });
    return () => {
      active = false;
    };
  }, [version]);

  return { overview, error, refresh };
}

/** Pastille du menu : nombre de mots à réviser aujourd'hui. */
export function useDueCount(): number | null {
  return useReviewOverview().overview?.due.length ?? null;
}
