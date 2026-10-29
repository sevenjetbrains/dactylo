/** Panneau « Profils » de la page Réglages (F-50) : créer, renommer, choisir la
 * disposition clavier, activer, supprimer. */

import { useCallback, useEffect, useId, useState, type FormEvent } from 'react';
import { useTranslation } from 'react-i18next';

import { commands, unwrap, type Layout, type Profile } from '@/shared/api';
import { Button } from '@/shared/ui/button';

/** Dispositions proposées (F-08). */
const LAYOUTS: Layout[] = ['azerty', 'qwerty', 'bepo'];

/** Classes d'un champ. */
const control =
  'h-9 rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring';

/** Message lisible d'une erreur de commande. */
function messageOf(e: unknown): string {
  return String((e as { message?: string }).message ?? e);
}

/** Recharge l'application sur l'accueil : tout repart du profil actif. */
function reloadOnHome() {
  window.location.hash = '#/';
  window.location.reload();
}

/** Une ligne : nom et disposition modifiables, activer, supprimer (avec confirmation). */
function ProfileRow({
  profile,
  active,
  onChanged,
  onError,
}: {
  profile: Profile;
  active: boolean;
  onChanged: () => void;
  onError: (message: string) => void;
}) {
  const { t } = useTranslation('profiles');
  const [name, setName] = useState(profile.name);
  const [confirming, setConfirming] = useState(false);
  const layout = profile.keyboardLayout as Layout;

  /** Enregistre le nom et la disposition. */
  const save = (next: { name?: string; layout?: Layout }) => {
    unwrap(commands.updateProfile(profile.id, next.name ?? name, next.layout ?? layout))
      .then(onChanged)
      .catch((e: unknown) => onError(messageOf(e)));
  };

  return (
    <li className="flex flex-wrap items-center gap-3 py-3">
      <input
        aria-label={t('nameOf', { name: profile.name })}
        value={name}
        onChange={(e) => setName(e.target.value)}
        onBlur={() => name.trim() && name !== profile.name && save({ name })}
        className={control}
      />
      <select
        aria-label={t('layoutOf', { name: profile.name })}
        value={layout}
        onChange={(e) => save({ layout: e.target.value as Layout })}
        className={control}
      >
        {LAYOUTS.map((l) => (
          <option key={l} value={l}>
            {t(`layouts.${l}`)}
          </option>
        ))}
      </select>
      {active ? (
        <span className="text-sm font-semibold text-primary">{t('active')}</span>
      ) : (
        <Button
          size="sm"
          variant="outline"
          onClick={() =>
            unwrap(commands.switchProfile(profile.id))
              .then(reloadOnHome)
              .catch((e: unknown) => onError(messageOf(e)))
          }
        >
          {t('activate')}
        </Button>
      )}
      {/* Suppression en deux temps : toutes les données du profil partent avec lui. */}
      {confirming ? (
        <span className="flex items-center gap-2 text-sm">
          {t('confirmDelete', { name: profile.name })}
          <Button
            size="sm"
            className="bg-destructive text-primary-foreground hover:bg-destructive/90"
            onClick={() =>
              unwrap(commands.deleteProfile(profile.id))
                .then(() => (active ? reloadOnHome() : onChanged()))
                .catch((e: unknown) => onError(messageOf(e)))
            }
          >
            {t('delete')}
          </Button>
          <Button size="sm" variant="ghost" onClick={() => setConfirming(false)}>
            {t('cancel')}
          </Button>
        </span>
      ) : (
        <Button
          size="sm"
          variant="ghost"
          aria-label={t('deleteNamed', { name: profile.name })}
          onClick={() => setConfirming(true)}
        >
          {t('delete')}
        </Button>
      )}
    </li>
  );
}

/** Liste des profils et création d'un nouveau. */
export default function ProfilesPanel() {
  const { t } = useTranslation('profiles');
  const id = useId();
  const [profiles, setProfiles] = useState<Profile[] | null>(null);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [name, setName] = useState('');
  const [layout, setLayout] = useState<Layout>('azerty');
  const [error, setError] = useState<string | null>(null);
  // Chaque incrément recharge la liste.
  const [version, setVersion] = useState(0);
  const reload = useCallback(() => setVersion((v) => v + 1), []);

  useEffect(() => {
    let active = true;
    Promise.all([unwrap(commands.listProfiles()), unwrap(commands.currentProfile())])
      .then(([list, current]) => {
        if (!active) return;
        setProfiles(list);
        setActiveId(current.id);
      })
      .catch((e: unknown) => active && setError(messageOf(e)));
    return () => {
      active = false;
    };
  }, [version]);

  /** Crée un profil. */
  const create = (e: FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;
    unwrap(commands.createProfile(name, layout))
      .then(() => {
        setName('');
        reload();
      })
      .catch((err: unknown) => setError(messageOf(err)));
  };

  return (
    <div className="flex flex-col gap-4">
      {error && (
        <p role="alert" className="text-sm text-destructive">
          {t('common:error.generic', { message: error })}
        </p>
      )}
      {profiles === null ? (
        <p>{t('common:loading')}</p>
      ) : (
        <ul className="flex flex-col divide-y divide-border">
          {profiles.map((p) => (
            <ProfileRow
              key={`${p.id}-${p.name}-${p.keyboardLayout}`}
              profile={p}
              active={p.id === activeId}
              onChanged={reload}
              onError={setError}
            />
          ))}
        </ul>
      )}
      <form onSubmit={create} className="flex flex-wrap items-end gap-3 text-sm">
        <label htmlFor={`${id}-name`} className="flex flex-col gap-1">
          {t('newName')}
          <input
            id={`${id}-name`}
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder={t('placeholder')}
            className={control}
          />
        </label>
        <label htmlFor={`${id}-layout`} className="flex flex-col gap-1">
          {t('layout')}
          <select
            id={`${id}-layout`}
            value={layout}
            onChange={(e) => setLayout(e.target.value as Layout)}
            className={control}
          >
            {LAYOUTS.map((l) => (
              <option key={l} value={l}>
                {t(`layouts.${l}`)}
              </option>
            ))}
          </select>
        </label>
        <Button type="submit" size="sm" className="h-9">
          {t('create')}
        </Button>
      </form>
    </div>
  );
}
