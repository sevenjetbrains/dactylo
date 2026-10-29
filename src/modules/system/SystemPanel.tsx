/** Panneau « Données » de la page Réglages : sauvegarde et restauration (F-52). */

import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

import {
  checkForUpdate,
  chooseOpenFile,
  chooseSaveFile,
  commands,
  unwrap,
  type AvailableUpdate,
} from '@/shared/api';
import { Button } from '@/shared/ui/button';

import { backupFilename } from './filename';

/** Extensions proposées par les boîtes de dialogue. */
const EXTENSIONS = ['dactylo', 'db'];

/** Message lisible d'une erreur de commande. */
function messageOf(e: unknown): string {
  return String((e as { message?: string }).message ?? e);
}

/** État de la vérification des mises à jour. */
type UpdateState =
  | { kind: 'idle' }
  | { kind: 'checking' }
  | { kind: 'upToDate' }
  | { kind: 'available'; update: AvailableUpdate }
  | { kind: 'installing' }
  | { kind: 'failed'; message: string };

/** Mises à jour en opt-in (ADR 0025) : case à cocher, vérification et installation. */
function UpdatesSection() {
  const { t } = useTranslation('system');
  const [onStart, setOnStart] = useState<boolean | null>(null);
  const [state, setState] = useState<UpdateState>({ kind: 'idle' });

  // Réglage enregistré ; aucune requête réseau n'est faite ici.
  useEffect(() => {
    let active = true;
    unwrap(commands.getUpdateCheck())
      .then((v) => active && setOnStart(v))
      .catch(() => active && setOnStart(false));
    return () => {
      active = false;
    };
  }, []);

  /** Interroge le serveur, sur demande explicite. */
  const checkNow = async () => {
    setState({ kind: 'checking' });
    try {
      const update = await checkForUpdate();
      setState(update ? { kind: 'available', update } : { kind: 'upToDate' });
    } catch (e) {
      setState({ kind: 'failed', message: messageOf(e) });
    }
  };

  /** Télécharge, installe et redémarre. */
  const install = async (update: AvailableUpdate) => {
    setState({ kind: 'installing' });
    try {
      await update.install();
    } catch (e) {
      setState({ kind: 'failed', message: messageOf(e) });
    }
  };

  return (
    <section
      aria-labelledby="updates-title"
      className="flex flex-col gap-3 border-t border-border pt-4"
    >
      <h3 id="updates-title" className="font-semibold">
        {t('updates.title')}
      </h3>
      <p className="text-muted-foreground">{t('updates.privacy')}</p>
      <label className="flex items-center gap-2">
        <input
          type="checkbox"
          checked={onStart ?? false}
          disabled={onStart === null}
          onChange={(e) => {
            const enabled = e.target.checked;
            setOnStart(enabled);
            void unwrap(commands.saveUpdateCheck(enabled)).catch(() => setOnStart(!enabled));
          }}
          className="accent-primary"
        />
        {t('updates.onStart')}
      </label>
      <div className="flex flex-wrap items-center gap-3">
        <Button
          variant="outline"
          disabled={state.kind === 'checking' || state.kind === 'installing'}
          onClick={() => void checkNow()}
        >
          {t('updates.checkNow')}
        </Button>
        {state.kind === 'available' && (
          <Button onClick={() => void install(state.update)}>
            {t('updates.install', { version: state.update.version })}
          </Button>
        )}
      </div>
      <p role="status" className="text-muted-foreground">
        {state.kind === 'checking' && t('updates.checking')}
        {state.kind === 'upToDate' && t('updates.upToDate')}
        {state.kind === 'available' && t('updates.available', { version: state.update.version })}
        {state.kind === 'installing' && t('updates.installing')}
      </p>
      {state.kind === 'available' && state.update.notes && (
        <p className="whitespace-pre-wrap">{state.update.notes}</p>
      )}
      {state.kind === 'failed' && (
        <p role="alert" className="text-destructive">
          {t('updates.failed', { message: state.message })}
        </p>
      )}
    </section>
  );
}

/** Sauvegarde dans un fichier, et restauration avec confirmation et redémarrage. */
export default function SystemPanel() {
  const { t } = useTranslation('system');
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const filters = [{ name: t('filter'), extensions: EXTENSIONS }];

  /** Sauvegarde toute la base dans le fichier choisi. */
  const backup = async () => {
    setError(null);
    setMessage(null);
    try {
      const path = await chooseSaveFile(backupFilename(), filters);
      if (!path) return;
      await unwrap(commands.backupDatabase(path));
      setMessage(t('saved', { path }));
    } catch (e) {
      setError(messageOf(e));
    }
  };

  /** Restaure la sauvegarde choisie ; l'application redémarre si elle est valide. */
  const restore = async () => {
    setError(null);
    setMessage(null);
    setConfirming(false);
    try {
      const path = await chooseOpenFile(filters);
      if (!path) return;
      setMessage(t('restoring'));
      await unwrap(commands.restoreDatabase(path));
    } catch (e) {
      setMessage(null);
      setError(messageOf(e));
    }
  };

  return (
    <div className="flex flex-col gap-4 text-sm">
      <p className="text-muted-foreground">{t('intro')}</p>
      <div className="flex flex-wrap items-center gap-3">
        <Button variant="outline" onClick={() => void backup()}>
          {t('backup')}
        </Button>
        {confirming ? (
          <span className="flex flex-wrap items-center gap-2">
            {t('confirmRestore')}
            <Button size="sm" onClick={() => void restore()}>
              {t('chooseFile')}
            </Button>
            <Button size="sm" variant="ghost" onClick={() => setConfirming(false)}>
              {t('cancel')}
            </Button>
          </span>
        ) : (
          <Button variant="outline" onClick={() => setConfirming(true)}>
            {t('restore')}
          </Button>
        )}
      </div>
      {message && <p role="status">{message}</p>}
      {error && (
        <p role="alert" className="text-destructive">
          {t('common:error.generic', { message: error })}
        </p>
      )}
      <UpdatesSection />
    </div>
  );
}
