/** Bandeau de lecture de la dictée : lecture / pause, segments, répétitions (F-14). */

import { Pause, Play, Repeat, SkipBack } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { cn } from '@/shared/lib/utils';

import { useDictationStore } from './store';

/** Classes d'un bouton clair sur le bandeau bleu nuit. */
const ghost =
  'inline-flex h-11 items-center gap-2 rounded-lg border border-sidebar-accent px-4 text-sm font-semibold text-sidebar-foreground outline-none hover:bg-sidebar-accent focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-50';

/** Bandeau bleu nuit de la maquette « Mode Dictée ». */
export function DictationBanner() {
  const { t } = useTranslation('dictation');
  const session = useDictationStore((s) => s.session);
  const status = useDictationStore((s) => s.status);
  const current = useDictationStore((s) => s.current);
  const repeats = useDictationStore((s) => s.repeats);
  const settings = useDictationStore((s) => s.settings);
  if (!session) return null;

  const total = session.segmentWords.length;
  const store = useDictationStore.getState();
  // Bouton principal : commencer (ou continuer après une reprise), pause, reprise.
  const playing = status === 'speaking' || status === 'listening';
  const onMain = () => {
    if (status === 'ready') void store.play(current + 1);
    else store.togglePause();
  };
  const started = current >= 0;
  const shown = Math.max(0, current) + 1;
  const used = started ? (repeats[current] ?? 0) : 0;
  const max = settings?.maxRepeats ?? null;

  return (
    <section
      aria-label={t('banner.label')}
      className="flex flex-col gap-5 rounded-2xl bg-sidebar p-6 text-sidebar-foreground shadow-xs"
    >
      <div className="flex items-center gap-5">
        <button
          type="button"
          onClick={onMain}
          disabled={status === 'ended' || status === 'finishing'}
          aria-label={
            status === 'ready'
              ? t(started ? 'banner.continue' : 'banner.start')
              : playing
                ? t('banner.pause')
                : t('banner.resume')
          }
          className="grid size-16 shrink-0 place-items-center rounded-full bg-accent text-accent-foreground outline-none hover:bg-accent/90 focus-visible:ring-2 focus-visible:ring-accent focus-visible:ring-offset-2 focus-visible:ring-offset-sidebar disabled:opacity-50"
        >
          {playing ? (
            <Pause aria-hidden className="size-6" />
          ) : (
            <Play aria-hidden className="size-6" />
          )}
        </button>
        <div className="flex min-w-0 flex-1 flex-col gap-3">
          <div className="flex flex-wrap justify-between gap-2 text-sm text-sidebar-muted">
            <p role="status">
              {t('banner.segment', { current: shown, total })} · {t(`banner.status.${status}`)}
            </p>
            <p>
              {max === null
                ? t('banner.repeatsUnlimited', { used })
                : t('banner.repeatsLimited', { used, max })}
            </p>
          </div>
          {/* Un trait par segment : lus, en cours, à venir. */}
          <ol aria-hidden className="flex gap-1">
            {session.segmentWords.map((_, i) => (
              <li
                key={i}
                className={cn(
                  'h-1.5 flex-1 rounded-full',
                  i < current ? 'bg-indigo-300' : i === current ? 'bg-accent' : 'bg-sidebar-accent',
                )}
              />
            ))}
          </ol>
        </div>
      </div>
      <div className="flex flex-wrap gap-3">
        <button
          type="button"
          className={ghost}
          disabled={!started || status === 'paused'}
          onClick={() => store.previous()}
        >
          <SkipBack aria-hidden className="size-4" />
          {t('banner.previous')}
        </button>
        <button
          type="button"
          className={ghost}
          disabled={!started || status === 'paused' || (max !== null && used >= max)}
          onClick={() => store.repeat()}
        >
          <Repeat aria-hidden className="size-4" />
          {t('banner.repeat')}
          <kbd className="font-mono text-xs text-sidebar-muted">{t('banner.repeatKey')}</kbd>
        </button>
      </div>
    </section>
  );
}
