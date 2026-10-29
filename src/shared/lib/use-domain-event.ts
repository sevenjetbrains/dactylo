/** Abonnement d'un composant aux événements de domaine relayés par le backend (ADR 0006). */

import { useEffect, useRef } from 'react';

import { events, type DomainEvent_Serialize as DomainEvent } from '@/shared/api';

/**
 * Appelle `handler` à chaque événement de domaine du type donné. L'abonnement
 * suit la vie du composant ; hors de Tauri (tests), il ne fait simplement rien.
 */
export function useDomainEvent(type: DomainEvent['type'], handler: () => void): void {
  // Le dernier gestionnaire est gardé sans relancer l'abonnement à chaque rendu.
  const latest = useRef(handler);
  useEffect(() => {
    latest.current = handler;
  });

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | null = null;
    // L'écoute est asynchrone : un démontage entre-temps la coupe dès qu'elle arrive.
    try {
      events.domainEventPayload
        .listen((e) => {
          if (e.payload.type === type) latest.current();
        })
        .then((stop) => {
          if (active) unlisten = stop;
          else stop();
        })
        .catch(() => undefined);
    } catch {
      // Pas de runtime Tauri : aucun événement ne viendra.
    }
    return () => {
      active = false;
      unlisten?.();
    };
  }, [type]);
}
