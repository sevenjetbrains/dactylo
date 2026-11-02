/** Page d'accueil. */

import { ClipboardPaste, FileText, Keyboard } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router';

import {
  commands,
  unwrap,
  type Dashboard_Serialize as Dashboard,
  type PendingSession,
  type Profile,
  type RecentSession_Serialize as RecentSession,
  type ReviewOverview,
} from '@/shared/api';
import { tzOffsetMinutes } from '@/shared/lib/dates';
import { useDomainEvent } from '@/shared/lib/use-domain-event';
import { cn } from '@/shared/lib/utils';
import { buttonVariants } from '@/shared/ui/button-variants';
import { Card, Eyebrow } from '@/shared/ui/card';
import { ProgressBar } from '@/shared/ui/progress-bar';

import { DictationCard } from './DictationCard';
import { RecentSessions } from './RecentSessions';
import { ReviewCard } from './ReviewCard';
import { StatTiles } from './StatTiles';

/** Sessions récentes chargées, parcourues ensuite par pages dans le tableau. */
const RECENT_COUNT = 50;

/** Date du jour en toutes lettres (« Mardi 27 octobre »), avec une majuscule initiale. */
function today(language: string): string {
  const text = new Intl.DateTimeFormat(language, {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
  }).format(new Date());
  return text.charAt(0).toUpperCase() + text.slice(1);
}

/**
 * Accueil : salutation, révision du jour (F-43), reprise d'une session interrompue
 * ou accès à la copie, dictée (F-10), tuiles de statistiques et sessions récentes.
 */
export default function HomePage() {
  const { t, i18n } = useTranslation('home');
  const [profile, setProfile] = useState<Profile | null>(null);
  const [pending, setPending] = useState<PendingSession | null>(null);
  const [overview, setOverview] = useState<ReviewOverview | null>(null);
  const [dashboard, setDashboard] = useState<Dashboard | null>(null);
  const [recent, setRecent] = useState<RecentSession[]>([]);
  const [error, setError] = useState<string | null>(null);
  // Chaque incrément recharge la révision du jour (mots changés en tâche de fond).
  const [wordsVersion, setWordsVersion] = useState(0);
  const reloadWords = useCallback(() => setWordsVersion((v) => v + 1), []);
  useDomainEvent('wordsUpdated', reloadWords);

  useEffect(() => {
    // `active` ignore les réponses arrivées après le démontage de la page.
    let active = true;
    const fail = (e: unknown) => active && setError(e instanceof Error ? e.message : String(e));
    // Profil courant, pour la salutation.
    unwrap(commands.currentProfile())
      .then((p) => active && setProfile(p))
      .catch(fail);
    // Une session interrompue est proposée à la reprise (NF-06).
    unwrap(commands.pendingSession())
      .then((p) => active && setPending(p))
      .catch(fail);
    // Tuiles (tous les temps) et dernières sessions.
    unwrap(commands.statsDashboard('all', tzOffsetMinutes()))
      .then((d) => active && setDashboard(d))
      .catch(fail);
    unwrap(commands.recentSessions(RECENT_COUNT))
      .then((r) => active && setRecent(r))
      .catch(fail);
    return () => {
      active = false;
    };
  }, []);

  // Révision du jour, rechargée quand les mots changent (fin de session, ajout…).
  useEffect(() => {
    let active = true;
    unwrap(commands.reviewOverview(tzOffsetMinutes()))
      .then((o) => active && setOverview(o))
      .catch((e: unknown) => active && setError(e instanceof Error ? e.message : String(e)));
    return () => {
      active = false;
    };
  }, [wordsVersion]);

  // Avancement de la session interrompue, en pourcentage entier.
  const progress = Math.round((pending?.progress ?? 0) * 100);

  return (
    <div className="mx-auto flex max-w-6xl flex-col gap-8">
      {/* En-tête : date, salutation et raccourci vers l'ajout d'un texte. */}
      <header className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="text-muted-foreground">{today(i18n.language)}</p>
          {/* Salutation générique tant que le profil n'est pas chargé. */}
          <h1 className="text-4xl font-bold tracking-tight">
            {profile ? t('greeting', { name: profile.name }) : t('greetingAnonymous')}
          </h1>
        </div>
        {/* Raccourcis vers les deux manières d'ajouter un texte. */}
        <div className="flex flex-wrap gap-3">
          <Link to="/library?view=paste" className={buttonVariants({ variant: 'outline' })}>
            <ClipboardPaste aria-hidden className="size-4" />
            {t('pasteText')}
          </Link>
          <Link to="/library?view=pdf" className={buttonVariants({ variant: 'outline' })}>
            <FileText aria-hidden className="size-4" />
            {t('importPdf')}
          </Link>
        </div>
      </header>

      {error && (
        <p role="alert" className="text-destructive">
          {t('common:error.generic', { message: error })}
        </p>
      )}

      {/* Cartes d'action : révision du jour, copie et dictée, comme sur la maquette. */}
      <div className="grid gap-6 lg:grid-cols-3">
        <ReviewCard overview={overview} />
        {pending ? (
          // Reprise d'une session interrompue : titre, avancement et bouton Continuer.
          <Card className="flex flex-col gap-4">
            <Eyebrow>
              <Keyboard aria-hidden className="size-4" />
              {t('resumeEyebrow')}
            </Eyebrow>
            <div>
              {/* Une révision n'a pas de titre stocké. */}
              <h2 className="text-xl font-bold">{pending.title || t('reviewResumeTitle')}</h2>
              {/* Exercice interrompu, pour un texte découpé (F-24). */}
              {pending.exercise.count > 1 && (
                <p className="text-sm text-muted-foreground">
                  {t('exercise', {
                    current: pending.exercise.index + 1,
                    count: pending.exercise.count,
                  })}
                </p>
              )}
            </div>
            <div className="mt-auto flex flex-col gap-2">
              <p className="flex justify-between text-sm">
                <span className="text-muted-foreground">{t('progress')}</span>
                <span className="font-semibold tabular-nums">{progress} %</span>
              </p>
              <ProgressBar value={pending.progress ?? 0} label={t('progress')} />
            </div>
            <Link to="/practice" className={cn(buttonVariants(), 'self-start')}>
              {t('continue')}
            </Link>
          </Card>
        ) : (
          // Sinon : démarrer une copie, ou choisir un texte dans la bibliothèque.
          <Card className="flex flex-col gap-4">
            <Eyebrow>
              <Keyboard aria-hidden className="size-4" />
              {t('startEyebrow')}
            </Eyebrow>
            <h2 className="text-xl font-bold">{t('startTitle')}</h2>
            <p className="text-muted-foreground">{t('startBody')}</p>
            <div className="mt-auto flex flex-wrap items-center gap-3">
              <Link to="/practice" className={buttonVariants()}>
                {t('startPractice')}
              </Link>
              <Link to="/library" className="text-sm font-semibold text-primary underline">
                {t('openLibrary')}
              </Link>
            </div>
          </Card>
        )}
        <DictationCard />
      </div>

      {/* Tuiles de statistiques et dernières sessions (F-33). */}
      {dashboard && <StatTiles dashboard={dashboard} />}
      <RecentSessions sessions={recent} />
    </div>
  );
}
