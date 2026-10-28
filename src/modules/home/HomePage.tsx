/** Page d'accueil. */

import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router';

import { commands, unwrap, type PendingSession, type Profile } from '@/shared/api';
import { Card, CardTitle } from '@/shared/ui/card';

/** Lien présenté comme un bouton principal. */
const linkClass =
  'inline-flex h-9 items-center rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring';

/** Accueil : salutation, reprise d'une session interrompue et accès à l'entraînement. */
export default function HomePage() {
  const { t } = useTranslation('home');
  const [profile, setProfile] = useState<Profile | null>(null);
  const [pending, setPending] = useState<PendingSession | null>(null);
  const [error, setError] = useState<string | null>(null);

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
    return () => {
      active = false;
    };
  }, []);

  return (
    <div className="mx-auto flex max-w-3xl flex-col gap-6">
      {/* Salutation générique tant que le profil n'est pas chargé. */}
      <h1 className="text-2xl font-semibold">
        {profile ? t('welcomeName', { name: profile.name }) : t('welcome')}
      </h1>
      {error && (
        <p role="alert" className="text-destructive">
          {t('common:error.generic', { message: error })}
        </p>
      )}
      {/* Carte de reprise, seulement s'il y a une session interrompue. */}
      {pending && (
        <Card>
          <CardTitle>{t('resumeTitle')}</CardTitle>
          <p className="mb-4 text-muted-foreground">
            {t('resumeBody', {
              title: pending.title,
              progress: Math.round((pending.progress ?? 0) * 100),
            })}
          </p>
          <Link to="/practice" className={linkClass}>
            {t('resumeAction')}
          </Link>
        </Card>
      )}
      {/* Accès à l'entraînement et à la bibliothèque. */}
      <Card>
        <CardTitle>{t('startTitle')}</CardTitle>
        <p className="mb-4 text-muted-foreground">{t('startBody')}</p>
        <div className="flex flex-wrap gap-3">
          <Link to="/practice" className={linkClass}>
            {t('startPractice')}
          </Link>
          <Link to="/library" className="inline-flex h-9 items-center px-2 text-sm underline">
            {t('openLibrary')}
          </Link>
        </div>
      </Card>
    </div>
  );
}
