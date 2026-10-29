/** Page du mode Dictée (F-10 à F-17). */

import { EyeOff } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useNavigate, useSearchParams } from 'react-router';

import {
  commands,
  events,
  unwrap,
  type ApiError,
  type DictationSettings_Serialize as DictationSettings,
  type DictationSummary_Serialize as Summary,
  type PendingDictation,
} from '@/shared/api';
import { Button } from '@/shared/ui/button';
import { Card, CardTitle, Eyebrow } from '@/shared/ui/card';

import { DictationBanner } from './DictationBanner';
import { DictationCorrection } from './DictationCorrection';
import { DictationSidebar } from './DictationSidebar';
import { activeMs, useDictationStore } from './store';
import { countWords, heardWords } from './words';

/** Intervalle des sauvegardes du brouillon (NF-06 : toutes les 10 s). */
const DRAFT_EVERY_MS = 10_000;

/** Étape de la page. */
type Phase =
  | { kind: 'loading' }
  | { kind: 'offer'; pending: PendingDictation }
  | { kind: 'empty' }
  | { kind: 'error'; message: string }
  | { kind: 'session' }
  | { kind: 'summary'; summary: Summary };

/** L'erreur vient-elle du backend (code + message) ? */
function isApiError(e: unknown): e is ApiError {
  return typeof e === 'object' && e !== null && 'code' in e && 'message' in e;
}

/** Message lisible d'une erreur quelconque. */
function messageOf(e: unknown): string {
  return isApiError(e) || e instanceof Error ? e.message : String(e);
}

/** Écoute la fin des lectures du moteur vocal ; hors de Tauri, ne fait rien. */
function useSpeechFinished(handler: (utterance: number) => void) {
  const latest = useRef(handler);
  useEffect(() => {
    latest.current = handler;
  });
  useEffect(() => {
    let active = true;
    let stop: (() => void) | null = null;
    try {
      events.speechFinished
        .listen((e) => latest.current(e.payload.utterance))
        .then((s) => {
          if (active) stop = s;
          else s();
        })
        .catch(() => undefined);
    } catch {
      // Pas de runtime Tauri : aucun événement ne viendra.
    }
    return () => {
      active = false;
      stop?.();
    };
  }, []);
}

/** Dictée : lecture par segments, copie à l'aveugle, correction (F-10, F-16). */
export default function DictationPage() {
  const { t } = useTranslation('dictation');
  const [params] = useSearchParams();
  const navigate = useNavigate();
  // Texte et exercice demandés par l'URL (`?text=<id>&exercise=3`, à partir de 1).
  const requested = params.get('text');
  const exerciseParam = Number(params.get('exercise'));
  const requestedExercise = exerciseParam >= 1 ? exerciseParam - 1 : null;

  const [phase, setPhase] = useState<Phase>({ kind: 'loading' });
  const session = useDictationStore((s) => s.session);
  const settings = useDictationStore((s) => s.settings);
  const typed = useDictationStore((s) => s.typed);
  const status = useDictationStore((s) => s.status);
  const current = useDictationStore((s) => s.current);
  const error = useDictationStore((s) => s.error);
  const request = useRef(0);
  const lastDraft = useRef('');

  useSpeechFinished((u) => useDictationStore.getState().onSpeechFinished(u));

  /** Démarre une nouvelle dictée sur le texte (ou le plus récent) et l'exercice donnés. */
  const begin = useCallback(async (textId: string | null, exercise: number | null) => {
    const token = ++request.current;
    setPhase({ kind: 'loading' });
    try {
      const start = await unwrap(commands.startDictation(textId, exercise));
      if (token !== request.current) return;
      useDictationStore.getState().load(start);
      lastDraft.current = '';
      setPhase({ kind: 'session' });
    } catch (e) {
      if (token !== request.current) return;
      const noText = isApiError(e) && e.code === 'notFound' && textId === null;
      setPhase(noText ? { kind: 'empty' } : { kind: 'error', message: messageOf(e) });
    }
  }, []);

  // Ouverture : une dictée interrompue est proposée, sauf si un texte est demandé.
  useEffect(() => {
    const requests = request;
    const token = ++requests.current;
    unwrap(commands.pendingDictation())
      .then((pending) => {
        if (token !== requests.current) return;
        if (pending && requested === null) setPhase({ kind: 'offer', pending });
        else void begin(requested, requestedExercise);
      })
      .catch((e: unknown) => {
        if (token === requests.current) setPhase({ kind: 'error', message: messageOf(e) });
      });
    return () => {
      requests.current++;
    };
  }, [requested, requestedExercise, begin]);

  /** Enregistre le brouillon s'il a changé (NF-06). */
  const saveDraft = useCallback(() => {
    const s = useDictationStore.getState();
    if (!s.session || s.status === 'finishing' || s.typed === lastDraft.current) return;
    lastDraft.current = s.typed;
    void unwrap(commands.saveDictationDraft(s.session.sessionId, s.typed, activeMs(s.clock))).catch(
      () => undefined,
    );
  }, []);

  // Brouillon toutes les 10 s ; à la sortie de la page, brouillon puis silence.
  useEffect(() => {
    if (!session) return;
    const id = setInterval(saveDraft, DRAFT_EVERY_MS);
    return () => {
      clearInterval(id);
      saveDraft();
    };
  }, [session, saveDraft]);
  useEffect(
    () => () => {
      void unwrap(commands.stopSpeech()).catch(() => undefined);
      useDictationStore.getState().reset();
    },
    [],
  );

  // Raccourcis (F-14) : Ctrl+Espace répète, Ctrl+← segment précédent, Échap pause.
  useEffect(() => {
    if (phase.kind !== 'session') return;
    const onKeyDown = (e: KeyboardEvent) => {
      const store = useDictationStore.getState();
      const ctrl = e.ctrlKey || e.metaKey;
      if (ctrl && e.code === 'Space') {
        e.preventDefault();
        store.repeat();
      } else if (ctrl && e.key === 'ArrowLeft') {
        e.preventDefault();
        store.previous();
      } else if (e.key === 'Escape') {
        e.preventDefault();
        if (store.status === 'ready') void store.play(store.current + 1);
        else store.togglePause();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [phase.kind]);

  /** Nouveaux réglages : en vigueur dès le segment suivant, et gardés pour le profil. */
  const changeSettings = async (next: DictationSettings) => {
    const store = useDictationStore.getState();
    if (!store.session) return;
    try {
      // Avant la première lecture, un autre découpage relance la dictée.
      if (next.segmentation !== store.settings?.segmentation && store.current < 0) {
        await unwrap(commands.saveDictationSettings(next));
        await unwrap(commands.abandonDictation(store.session.sessionId, '', 0));
        await begin(store.session.textId, store.session.exercise.index);
        return;
      }
      store.setSettings(next);
      const effective = await unwrap(
        commands.updateDictationSettings(store.session.sessionId, next),
      );
      useDictationStore.getState().setSettings(effective);
      await unwrap(commands.saveDictationSettings(effective));
    } catch (e) {
      setPhase({ kind: 'error', message: messageOf(e) });
    }
  };

  /** Termine et corrige (F-16). */
  const finish = async () => {
    const store = useDictationStore.getState();
    if (!store.session) return;
    store.markFinishing();
    void unwrap(commands.stopSpeech()).catch(() => undefined);
    const { session: s, typed: text, clock } = useDictationStore.getState();
    try {
      const summary = await unwrap(
        commands.finishDictation(s?.sessionId ?? '', text, activeMs(clock)),
      );
      setPhase({ kind: 'summary', summary });
    } catch (e) {
      setPhase({ kind: 'error', message: messageOf(e) });
    }
  };

  /** Reprend la dictée interrompue proposée. */
  const resume = async (id: string) => {
    const token = ++request.current;
    setPhase({ kind: 'loading' });
    try {
      const start = await unwrap(commands.resumeDictation(id));
      if (token !== request.current) return;
      useDictationStore.getState().load(start);
      lastDraft.current = start.typed;
      setPhase({ kind: 'session' });
    } catch (e) {
      if (token === request.current) setPhase({ kind: 'error', message: messageOf(e) });
    }
  };

  /** Abandonne la dictée proposée et repart de zéro sur le même texte. */
  const discard = async (pending: PendingDictation) => {
    await unwrap(commands.abandonDictation(pending.sessionId, '', 0)).catch(() => undefined);
    await begin(pending.textId, pending.exercise.index);
  };

  /** Exercice sur les mots fautifs, tapé dans la Copie (ADR 0018). */
  const drill = async (words: string[]) => {
    try {
      const content = await unwrap(commands.wordsExercise(words, { type: 'repeated', times: 3 }));
      if (content) navigate('/practice', { state: { review: content } });
    } catch (e) {
      setPhase({ kind: 'error', message: messageOf(e) });
    }
  };

  // Correction : écran à part, sans le bandeau de lecture.
  if (phase.kind === 'summary') {
    const s = phase.summary;
    return (
      <div className="mx-auto max-w-6xl">
        <DictationCorrection
          summary={s}
          onRedo={() => void begin(s.textId, s.exercise.index)}
          onDrill={(words) => void drill(words)}
        />
      </div>
    );
  }

  const typedWords = countWords(typed);
  const nextAt = session && current >= 0 ? heardWords(session.segmentWords, current) : 0;

  return (
    <div className="mx-auto flex max-w-6xl flex-col gap-6">
      <header>
        <Eyebrow>
          {session ? t('eyebrow', { count: session.totalWords }) : t('menu')}
          {session &&
            session.exercise.count > 1 &&
            ` · ${t('exercise', { current: session.exercise.index + 1, count: session.exercise.count })}`}
        </Eyebrow>
        <h1 className="mt-1 text-3xl font-bold tracking-tight">
          {session && phase.kind === 'session' ? session.title : t('menu')}
        </h1>
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
              typed: phase.pending.typedWords,
              total: phase.pending.totalWords,
            })}
          </p>
          <div className="flex flex-wrap gap-3">
            <Button autoFocus onClick={() => void resume(phase.pending.sessionId)}>
              {t('resume.continue')}
            </Button>
            <Button variant="outline" onClick={() => void discard(phase.pending)}>
              {t('resume.discard')}
            </Button>
          </div>
        </Card>
      )}

      {phase.kind === 'session' && session && settings && (
        <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_21rem]">
          <div className="flex flex-col gap-6">
            <DictationBanner />
            {error && (
              <p role="alert" className="text-destructive">
                {t('speechError', { message: error })}
              </p>
            )}
            {/* Copie à l'aveugle : zone de texte libre, texte masqué (F-10). */}
            <Card className="flex flex-col gap-4">
              <div className="flex flex-wrap items-baseline justify-between gap-2">
                <label htmlFor="dictation-copy" className="text-lg font-semibold">
                  {t('copy.title')}
                </label>
                <p className="flex items-center gap-1 text-sm text-muted-foreground">
                  <EyeOff aria-hidden className="size-4" />
                  {t('copy.hidden')}
                </p>
              </div>
              <textarea
                id="dictation-copy"
                value={typed}
                onChange={(e) => useDictationStore.getState().setTyped(e.target.value)}
                disabled={status === 'finishing'}
                spellCheck={false}
                autoCorrect="off"
                autoCapitalize="off"
                autoComplete="off"
                lang="fr"
                rows={9}
                className="w-full resize-y rounded-xl border-2 border-primary bg-background p-4 font-mono text-lg leading-relaxed outline-none focus-visible:ring-4 focus-visible:ring-primary/20"
              />
              <div className="flex flex-wrap items-center justify-between gap-3">
                <p className="text-sm text-muted-foreground">
                  {t('copy.typed', { count: typedWords })}
                  {' · '}
                  {settings.advance === 'adaptive'
                    ? t('copy.adaptiveHint', { count: Math.max(0, nextAt - typedWords) })
                    : t('copy.automaticHint', { seconds: settings.pauseMs / 1000 })}
                </p>
                <Button onClick={() => void finish()} disabled={status === 'finishing'}>
                  {t('copy.finish')}
                </Button>
              </div>
            </Card>
            <p className="text-sm text-muted-foreground">{t('shortcuts')}</p>
          </div>
          <DictationSidebar
            settings={settings}
            segmentationLocked={current >= 0}
            onChange={(s) => void changeSettings(s)}
          />
        </div>
      )}
    </div>
  );
}
