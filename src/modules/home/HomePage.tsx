import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { commands, unwrap, type Profile } from '@/shared/api';
import { Card, CardTitle } from '@/shared/ui/card';

export default function HomePage() {
  const { t } = useTranslation('home');
  const [profile, setProfile] = useState<Profile | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    unwrap(commands.currentProfile())
      .then((p) => active && setProfile(p))
      .catch((e: unknown) => active && setError(e instanceof Error ? e.message : String(e)));
    return () => {
      active = false;
    };
  }, []);

  return (
    <div className="mx-auto flex max-w-3xl flex-col gap-6">
      <h1 className="text-2xl font-semibold">
        {profile ? t('welcomeName', { name: profile.name }) : t('welcome')}
      </h1>
      {error && (
        <p role="alert" className="text-destructive">
          {t('common:error.generic', { message: error })}
        </p>
      )}
      <Card>
        <CardTitle>{t('emptyTitle')}</CardTitle>
        <p className="text-muted-foreground">{t('emptyBody')}</p>
      </Card>
    </div>
  );
}
