/** Panneau « Données » de la page Réglages : sauvegarde et restauration (F-52). */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';

import { chooseOpenFile, chooseSaveFile, commands, unwrap } from '@/shared/api';
import { Button } from '@/shared/ui/button';

import { backupFilename } from './filename';

/** Extensions proposées par les boîtes de dialogue. */
const EXTENSIONS = ['dactylo', 'db'];

/** Message lisible d'une erreur de commande. */
function messageOf(e: unknown): string {
  return String((e as { message?: string }).message ?? e);
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
    </div>
  );
}
