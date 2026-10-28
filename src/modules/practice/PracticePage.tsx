import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useNavigate, useSearchParams } from 'react-router';

import {
  commands,
  unwrap,
  type ApiError,
  type PendingSession,
  type SessionSummary_Serialize as Summary,
} from '@/shared/api';
import { Button } from '@/shared/ui/button';
import { Card, CardTitle } from '@/shared/ui/card';

import { goalEndPosition, liveStats, type RawKey } from './engine/session';
import { formatNumber } from './format';
import { LiveBanner } from './LiveBanner';
import { SessionSummary } from './SessionSummary';
import { activeMs, usePracticeStore } from './store';
import { FALLBACK_COLUMNS } from './layout';
import { TypingArea } from './TypingArea';
import { useKeystrokeSync } from './use-keystroke-sync';

type Phase =
  | { kind: 'loading' }
  | { kind: 'offer'; pending: PendingSession }
  | { kind: 'empty' }
  | { kind: 'error'; message: string }
  | { kind: 'session' }
  | { kind: 'summary'; summary: Summary };

/** Rafraîchissement du bandeau et du chronomètre. */
const TICK_MS = 250;

function isApiError(e: unknown): e is ApiError {
  return typeof e === 'object' && e !== null && 'code' in e && 'message' in e;
}

function messageOf(e: unknown): string {
  return isApiError(e) || e instanceof Error ? e.message : String(e);
}

/** Mode Copie (F-01 à F-07, F-32) et reprise d'une session interrompue (NF-06). */
export default function PracticePage() {
  const { t, i18n } = useTranslation('practice');
  const [params] = useSearchParams();
  const requested = params.get('text');
  const navigate = useNavigate();

  const [phase, setPhase] = useState<Phase>({ kind: 'loading' });
  const [columns, setColumns] = useState<number>(FALLBACK_COLUMNS);
  const [lineStarts, setLineStarts] = useState<number[] | null>(null);
  const [now, setNow] = useState(() => performance.now());

  const session = usePracticeStore((s) => s.session);
  const engine = usePracticeStore((s) => s.engine);
  const version = usePracticeStore((s) => s.version);
  const status = usePracticeStore((s) => s.status);
  const clock = usePracticeStore((s) => s.clock);
  const sessionId = session?.sessionId ?? null;
  const flush = useKeystrokeSync(sessionId);

  // Chaque requête reçoit un jeton : une réponse arrivée trop tard est ignorée.
  const request = useRef(0);

  const begin = useCallback(async (textId: string | null) => {
    const token = ++request.current;
    setPhase({ kind: 'loading' });
    setLineStarts(null);
    try {
      const start = await unwrap(commands.startSession(textId));
      if (token !== request.current) return;
      usePracticeStore.getState().load(start);
      setPhase({ kind: 'session' });
    } catch (e) {
      if (token !== request.current) return;
      setPhase(
        isApiError(e) && e.code === 'notFound' && textId === null
          ? { kind: 'empty' }
          : { kind: 'error', message: messageOf(e) },
      );
    }
  }, []);

  // Ouverture : une session interrompue est proposée à la reprise avant tout.
  useEffect(() => {
    const requests = request;
    const token = ++requests.current;
    unwrap(commands.pendingSession())
      .then((pending) => {
        if (token !== requests.current) return;
        if (pending) setPhase({ kind: 'offer', pending });
        else void begin(requested);
      })
      .catch((e: unknown) => {
        if (token === requests.current) setPhase({ kind: 'error', message: messageOf(e) });
      });
    return () => {
      requests.current++;
    };
  }, [requested, begin]);

  // En quittant la page, les frappes en attente sont envoyées ; la session reste reprenable.
  const flushRef = useRef(flush);
  useEffect(() => {
    flushRef.current = flush;
  }, [flush]);
  useEffect(() => () => void flushRef.current(), []);

  // Retour à la ligne calculé par le cœur, à l'ouverture et au redimensionnement.
  useEffect(() => {
    if (!sessionId) return;
    let active = true;
    const timer = setTimeout(() => {
      unwrap(commands.wrapText(sessionId, columns))
        .then((starts) => active && setLineStarts(starts))
        .catch((e: unknown) => active && setPhase({ kind: 'error', message: messageOf(e) }));
    }, 50);
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [sessionId, columns]);

  // Chronomètre : bandeau et fin des sessions à durée fixe.
  useEffect(() => {
    if (status !== 'typing') return;
    const id = setInterval(() => {
      const t = performance.now();
      setNow(t);
      usePracticeStore.getState().checkGoal(t);
    }, TICK_MS);
    return () => clearInterval(id);
  }, [status]);

  // Fin de session : envoi des dernières frappes, puis calcul par le cœur.
  useEffect(() => {
    if (status !== 'finishing' || !sessionId) return;
    const token = request.current;
    void (async () => {
      try {
        await flush();
        const ms = activeMs(usePracticeStore.getState().clock, performance.now());
        const summary = await unwrap(commands.finishSession(sessionId, ms));
        usePracticeStore.getState().markDone();
        if (token === request.current) setPhase({ kind: 'summary', summary });
      } catch (e) {
        if (token === request.current) setPhase({ kind: 'error', message: messageOf(e) });
      }
    })();
  }, [status, sessionId, flush]);

  const onKey = useCallback((key: RawKey) => {
    usePracticeStore.getState().input(key, performance.now());
  }, []);

  /**
   * Abandonne la session en cours si elle n'est pas terminée. La session et sa
   * file sont prises tout de suite : l'abandon ne peut viser que cette session.
   */
  const dropCurrent = useCallback(() => {
    const { session: current, status: currentStatus, takeBatch } = usePracticeStore.getState();
    if (!current) return null;
    const token = ++request.current;
    let dropped: Promise<unknown> = Promise.resolve();
    if (currentStatus !== 'done') {
      takeBatch();
      dropped = unwrap(commands.abandonSession(current.sessionId)).catch(() => undefined);
    }
    return { textId: current.textId, token, dropped };
  }, []);

  const restart = useCallback(async () => {
    const current = dropCurrent();
    if (!current) return;
    await current.dropped;
    if (current.token === request.current) await begin(current.textId);
  }, [dropCurrent, begin]);

  const next = useCallback(async () => {
    const current = dropCurrent();
    if (!current) return;
    const [nextId] = await Promise.all([
      unwrap(commands.nextTextId(current.textId)).catch(() => null),
      current.dropped,
    ]);
    // Page quittée ou autre action lancée entre-temps.
    if (current.token !== request.current) return;
    const target = nextId ?? current.textId;
    if (target !== requested) navigate(`/practice?text=${encodeURIComponent(target)}`);
    else await begin(target);
  }, [dropCurrent, begin, navigate, requested]);

  // Raccourcis (F-07) : Échap = pause, Ctrl+R = recommencer, Ctrl+Entrée = texte suivant.
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      const ctrl = e.ctrlKey || e.metaKey;
      if (ctrl && e.key.toLowerCase() === 'r') {
        // Empêche aussi le rechargement de la fenêtre.
        e.preventDefault();
        void restart();
      } else if (ctrl && e.key === 'Enter') {
        e.preventDefault();
        void next();
      } else if (e.key === 'Escape') {
        const store = usePracticeStore.getState();
        if (store.status === 'typing' || store.status === 'paused') {
          e.preventDefault();
          store.togglePause(performance.now());
          if (store.status === 'typing') void flush();
        }
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [restart, next, flush]);

  const resume = async (id: string) => {
    const token = ++request.current;
    setPhase({ kind: 'loading' });
    try {
      const start = await unwrap(commands.resumeSession(id));
      if (token !== request.current) return;
      usePracticeStore.getState().load(start);
      setPhase({ kind: 'session' });
    } catch (e) {
      if (token === request.current) setPhase({ kind: 'error', message: messageOf(e) });
    }
  };

  const discard = async (id: string) => {
    await unwrap(commands.abandonSession(id)).catch(() => undefined);
    await begin(requested);
  };

  return (
    <div className="mx-auto flex max-w-4xl flex-col gap-6">
      <header className="flex flex-wrap items-baseline justify-between gap-2">
        <h1 className="text-2xl font-semibold">
          {phase.kind === 'session' || phase.kind === 'summary'
            ? (session?.title ?? t('title'))
            : t('title')}
        </h1>
        <p className="text-xs text-muted-foreground">{t('shortcuts')}</p>
      </header>

      {phase.kind === 'loading' && <p>{t('common:loading')}</p>}

      {phase.kind === 'error' && (
        <p role="alert" className="text-destructive">
          {t('common:error.generic', { message: phase.message })}
        </p>
      )}

      {phase.kind === 'empty' && (
        <Card>
          <CardTitle>{t('empty.title')}</CardTitle>
          <p className="mb-4 text-muted-foreground">{t('empty.body')}</p>
          <Link to="/library" className="underline">
            {t('empty.action')}
          </Link>
        </Card>
      )}

      {phase.kind === 'offer' && (
        <Card>
          <CardTitle>{t('resume.title')}</CardTitle>
          <p className="mb-4 text-muted-foreground">
            {t('resume.body', {
              title: phase.pending.title,
              progress: formatNumber((phase.pending.progress ?? 0) * 100, i18n.language),
            })}
          </p>
          <div className="flex flex-wrap gap-3">
            <Button autoFocus onClick={() => void resume(phase.pending.sessionId)}>
              {t('resume.continue')}
            </Button>
            <Button variant="outline" onClick={() => void discard(phase.pending.sessionId)}>
              {t('resume.discard')}
            </Button>
          </div>
        </Card>
      )}

      {phase.kind === 'session' && session && engine && (
        <>
          {session.settings.showBanner && (
            <LiveBanner
              stats={liveStats(
                engine.counters(),
                activeMs(clock, now),
                session.settings.goal,
                engine.position,
                goalEndPosition(session.settings.goal, engine.text.length, session.words),
              )}
              elapsedMs={activeMs(clock, now)}
            />
          )}
          {lineStarts ? (
            <TypingArea
              engine={engine}
              version={version}
              lineStarts={lineStarts}
              settings={session.settings}
              disabled={status !== 'typing'}
              onKey={onKey}
              onColumns={setColumns}
            />
          ) : (
            <p>{t('common:loading')}</p>
          )}
          {status === 'paused' && (
            <p role="status" className="text-center font-medium">
              {t('paused')}
            </p>
          )}
          {status === 'finishing' && (
            <p role="status" className="text-center text-muted-foreground">
              {t('saving')}
            </p>
          )}
        </>
      )}

      {phase.kind === 'summary' && (
        <SessionSummary
          summary={phase.summary}
          onRestart={() => void restart()}
          onNext={() => void next()}
        />
      )}
    </div>
  );
}
