import { useCallback, useEffect, useRef } from 'react';

import { commands, unwrap } from '@/shared/api';

import { usePracticeStore } from './store';

/** Intervalle d'envoi des frappes (§3.2) ; bien en deçà des 10 s de NF-06. */
export const SYNC_INTERVAL_MS = 2_000;

/**
 * Envoie les frappes en attente toutes les 2 s. `flush` force un envoi
 * (pause, fin de session, sortie de la page).
 *
 * Les envois se suivent un par un et chaque lot n'est prélevé qu'au moment de
 * son envoi : un lot refusé est remis en tête de file avant que le suivant
 * parte, ce qui garde l'ordre des frappes côté backend.
 */
export function useKeystrokeSync(sessionId: string | null) {
  const inflight = useRef<Promise<void>>(Promise.resolve());

  const flush = useCallback(() => {
    inflight.current = inflight.current.then(async () => {
      const store = usePracticeStore.getState();
      // La file appartient à la session chargée : ne rien envoyer pour une autre.
      if (!sessionId || store.session?.sessionId !== sessionId) return;
      const batch = store.takeBatch();
      if (batch.length === 0) return;
      try {
        await unwrap(commands.recordKeystrokes(sessionId, batch));
      } catch {
        if (usePracticeStore.getState().session?.sessionId === sessionId) {
          usePracticeStore.getState().requeue(batch);
        }
      }
    });
    return inflight.current;
  }, [sessionId]);

  useEffect(() => {
    const id = setInterval(() => void flush(), SYNC_INTERVAL_MS);
    return () => clearInterval(id);
  }, [flush]);

  return flush;
}
