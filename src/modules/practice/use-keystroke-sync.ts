/** Envoi périodique des frappes au backend. */

import { useCallback, useEffect } from 'react';

import { commands, unwrap } from '@/shared/api';

import { usePracticeStore } from './store';

/** Intervalle d'envoi des frappes (§3.2) ; bien en deçà des 10 s de NF-06. */
export const SYNC_INTERVAL_MS = 2_000;

/**
 * Chaîne des envois, commune à toute la page : chaque envoi attend la fin du
 * précédent, même d'une page Copie démontée entre-temps (navigation).
 */
let chain: Promise<void> = Promise.resolve();

/**
 * Attend la fin des envois en cours. À appeler avant de démarrer une session : le
 * démarrage abandonne la précédente, dont les dernières frappes doivent être arrivées
 * (F-31, ADR 0014).
 */
export function waitForSync(): Promise<void> {
  return chain;
}

/**
 * Envoie les frappes en attente toutes les 2 s. `flush` force un envoi
 * (pause, fin de session, sortie de la page).
 *
 * Les envois se suivent un par un et chaque lot n'est prélevé qu'au moment de
 * son envoi : un lot refusé est remis en tête de file avant que le suivant
 * parte, ce qui garde l'ordre des frappes côté backend.
 */
export function useKeystrokeSync(sessionId: string | null) {
  const flush = useCallback(() => {
    chain = chain.then(async () => {
      const store = usePracticeStore.getState();
      // La file appartient à la session chargée : ne rien envoyer pour une autre.
      if (!sessionId || store.session?.sessionId !== sessionId) return;
      // Le lot n'est retiré de la file qu'au moment de partir ; rien à faire s'il est vide.
      const batch = store.takeBatch();
      if (batch.length === 0) return;
      try {
        await unwrap(commands.recordKeystrokes(sessionId, batch));
      } catch {
        // Échec : le lot revient en tête de file, sauf si la session a changé entre-temps.
        if (usePracticeStore.getState().session?.sessionId === sessionId) {
          usePracticeStore.getState().requeue(batch);
        }
      }
    });
    return chain;
  }, [sessionId]);

  // Envoi automatique à intervalle régulier, arrêté au démontage.
  useEffect(() => {
    const id = setInterval(() => void flush(), SYNC_INTERVAL_MS);
    return () => clearInterval(id);
  }, [flush]);

  return flush;
}
