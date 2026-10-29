/** Carte « Dictée » de l'accueil (F-10) : reprise ou lancement, réglages du profil. */

import { Volume2 } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router';

import {
  commands,
  unwrap,
  type DictationSettings_Serialize as DictationSettings,
  type PendingDictation,
} from '@/shared/api';
import { formatNumber } from '@/shared/lib/format';
import { cn } from '@/shared/lib/utils';
import { buttonVariants } from '@/shared/ui/button-variants';
import { Card, Eyebrow } from '@/shared/ui/card';

/** Dictée interrompue à reprendre, ou invitation à en lancer une, avec les réglages en pastilles. */
export function DictationCard() {
  const { t, i18n } = useTranslation('home');
  const [pending, setPending] = useState<PendingDictation | null>(null);
  const [settings, setSettings] = useState<DictationSettings | null>(null);

  // Dictée interrompue et réglages du profil ; une erreur laisse la carte générique.
  useEffect(() => {
    let active = true;
    unwrap(commands.pendingDictation())
      .then((p) => active && setPending(p))
      .catch(() => undefined);
    unwrap(commands.getDictationSettings())
      .then((s) => active && setSettings(s))
      .catch(() => undefined);
    return () => {
      active = false;
    };
  }, []);

  // Pastilles des réglages : découpage, vitesse, ponctuation lue.
  const chips = settings
    ? [
        settings.segmentation === 'group'
          ? t('dictation.group', { count: settings.groupSize })
          : t(`dictation.${settings.segmentation}`),
        t('dictation.rate', { value: formatNumber(settings.rate ?? 1, i18n.language, 1) }),
        ...(settings.readPunctuation ? [t('dictation.punctuation')] : []),
      ]
    : [];

  return (
    <Card className="flex flex-col gap-4">
      <Eyebrow>
        <Volume2 aria-hidden className="size-4" />
        {t('dictation.eyebrow')}
      </Eyebrow>
      <div>
        <h2 className="text-xl font-bold">{pending ? pending.title : t('dictation.title')}</h2>
        <p className="text-sm text-muted-foreground">
          {pending
            ? t('dictation.resumeBody', { typed: pending.typedWords, total: pending.totalWords })
            : t('dictation.body')}
        </p>
      </div>
      {chips.length > 0 && (
        <ul className="flex flex-wrap gap-2">
          {chips.map((c) => (
            <li
              key={c}
              className="rounded-full bg-primary/10 px-3 py-1 text-sm font-medium text-primary"
            >
              {c}
            </li>
          ))}
        </ul>
      )}
      <Link
        to="/dictation"
        className={cn(
          buttonVariants({ variant: 'outline' }),
          'mt-auto self-start border-primary text-primary',
        )}
      >
        {pending ? t('dictation.resume') : t('dictation.start')}
      </Link>
    </Card>
  );
}
