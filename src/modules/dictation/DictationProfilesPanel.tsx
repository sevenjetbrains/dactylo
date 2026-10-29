/** Panneau « Dictée » de la page Réglages : profils de dictée enregistrés (F-17). */

import { useCallback, useEffect, useId, useState, type FormEvent } from 'react';
import { useTranslation } from 'react-i18next';

import {
  commands,
  unwrap,
  type DictationProfile_Serialize as DictationProfile,
} from '@/shared/api';
import { formatNumber } from '@/shared/lib/format';
import { Button } from '@/shared/ui/button';

/** Message lisible d'une erreur de commande. */
function messageOf(e: unknown): string {
  return String((e as { message?: string }).message ?? e);
}

/** Liste des profils, enregistrement des réglages courants sous un nom, suppression. */
export default function DictationProfilesPanel() {
  const { t, i18n } = useTranslation('dictation');
  const id = useId();
  const [profiles, setProfiles] = useState<DictationProfile[] | null>(null);
  const [name, setName] = useState('');
  const [error, setError] = useState<string | null>(null);
  // Chaque incrément recharge la liste.
  const [version, setVersion] = useState(0);
  const reload = useCallback(() => setVersion((v) => v + 1), []);

  useEffect(() => {
    let active = true;
    unwrap(commands.listDictationProfiles())
      .then((p) => active && setProfiles(p))
      .catch((e: unknown) => active && setError(messageOf(e)));
    return () => {
      active = false;
    };
  }, [version]);

  /** Enregistre les réglages courants de la dictée sous le nom saisi. */
  const save = async (e: FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;
    try {
      const current = await unwrap(commands.getDictationSettings());
      await unwrap(commands.saveDictationProfile(null, name, current));
      setName('');
      reload();
    } catch (err) {
      setError(messageOf(err));
    }
  };

  /** Rend un profil actif : ses réglages deviennent ceux des prochaines dictées. */
  const apply = (p: DictationProfile) => {
    unwrap(commands.saveDictationSettings(p.settings)).catch((e: unknown) =>
      setError(messageOf(e)),
    );
  };

  /** Supprime un profil. */
  const remove = (p: DictationProfile) => {
    unwrap(commands.deleteDictationProfile(p.id))
      .then(reload)
      .catch((e: unknown) => setError(messageOf(e)));
  };

  return (
    <div className="flex flex-col gap-4 text-sm">
      {error && (
        <p role="alert" className="text-destructive">
          {t('common:error.generic', { message: error })}
        </p>
      )}
      {profiles === null ? (
        <p>{t('common:loading')}</p>
      ) : profiles.length === 0 ? (
        <p className="text-muted-foreground">{t('profiles.empty')}</p>
      ) : (
        <ul className="flex flex-col divide-y divide-border">
          {profiles.map((p) => (
            <li key={p.id} className="flex flex-wrap items-center justify-between gap-3 py-2">
              <span>
                <span className="font-semibold">{p.name}</span>
                <span className="ml-2 text-muted-foreground">
                  {t('profiles.summary', {
                    rate: formatNumber(p.settings.rate ?? 1, i18n.language, 1),
                    mode: t(`sidebar.modes.${p.settings.segmentation}`),
                  })}
                </span>
              </span>
              <span className="flex gap-2">
                <Button size="sm" variant="outline" onClick={() => apply(p)}>
                  {t('profiles.use')}
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => remove(p)}
                  aria-label={t('profiles.deleteNamed', { name: p.name })}
                >
                  {t('profiles.delete')}
                </Button>
              </span>
            </li>
          ))}
        </ul>
      )}
      <form onSubmit={(e) => void save(e)} className="flex flex-wrap items-end gap-3">
        <label htmlFor={`${id}-name`} className="flex flex-col gap-1">
          {t('profiles.name')}
          <input
            id={`${id}-name`}
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder={t('profiles.placeholder')}
            className="h-9 rounded-md border border-border bg-background px-2 outline-none focus-visible:ring-2 focus-visible:ring-ring"
          />
        </label>
        <Button type="submit" size="sm" className="h-9">
          {t('profiles.save')}
        </Button>
      </form>
    </div>
  );
}
