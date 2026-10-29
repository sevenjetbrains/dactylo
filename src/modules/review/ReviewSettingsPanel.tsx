/** Panneau des réglages de la révision. */

import { useEffect, useId, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { commands, unwrap, type ReviewSettings_Serialize as ReviewSettings } from '@/shared/api';
import { formatNumber } from '@/shared/lib/format';

/** Seuils de lenteur proposés (F-40) ; 1,5 par défaut. */
const FACTORS = [1.25, 1.5, 1.75, 2, 2.5, 3];
/** Seuil par défaut, si le backend n'en renvoie pas de lisible. */
const DEFAULT_FACTOR = 1.5;

/** Panneau « Révision » de la page Réglages : seuil de lenteur des mots (F-40). */
export default function ReviewSettingsPanel() {
  const { t, i18n } = useTranslation('review');
  const id = useId();
  const [settings, setSettings] = useState<ReviewSettings | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Chargement des réglages ; `active` ignore une réponse arrivée après le démontage.
  useEffect(() => {
    let active = true;
    unwrap(commands.getReviewSettings())
      .then((s) => active && setSettings(s))
      .catch((e: unknown) => active && setError(String((e as { message?: string }).message ?? e)));
    return () => {
      active = false;
    };
  }, []);

  if (error) {
    return (
      <p role="alert" className="text-destructive">
        {t('common:error.generic', { message: error })}
      </p>
    );
  }
  if (!settings) return <p>{t('common:loading')}</p>;

  /** Affiche le choix tout de suite, puis l'enregistre ; le backend renvoie la valeur bornée. */
  const update = (slowFactor: number) => {
    const next = { ...settings, slowFactor };
    setSettings(next);
    unwrap(commands.saveReviewSettings(next))
      .then(setSettings)
      .catch((e: unknown) => setError(String((e as { message?: string }).message ?? e)));
  };

  // Un seuil enregistré hors de la liste (réglé ailleurs) reste proposé.
  const current = settings.slowFactor ?? DEFAULT_FACTOR;
  const factors = FACTORS.includes(current) ? FACTORS : [...FACTORS, current].sort((a, b) => a - b);

  return (
    <div className="flex flex-col gap-1 text-sm">
      <label htmlFor={`${id}-slow`}>{t('settings.slowFactor')}</label>
      <select
        id={`${id}-slow`}
        className="h-9 max-w-xs rounded-md border border-border bg-background px-2 outline-none focus-visible:ring-2 focus-visible:ring-ring"
        value={current}
        onChange={(e) => update(Number(e.target.value))}
      >
        {factors.map((f) => (
          <option key={f} value={f}>
            {t('settings.factor', { value: formatNumber(f, i18n.language, f % 1 === 0 ? 0 : 2) })}
          </option>
        ))}
      </select>
      <p className="text-muted-foreground">{t('settings.slowFactorHint')}</p>
    </div>
  );
}
