/** Panneau des réglages d'entraînement. */

import { useEffect, useId, useState } from 'react';
import { useTranslation } from 'react-i18next';

import {
  commands,
  unwrap,
  type PracticeSettings_Serialize as PracticeSettings,
  type SessionGoal,
} from '@/shared/api';

/** Durées proposées, en minutes (F-06). */
const DURATIONS = [1, 2, 5, 10];
/** Nombres de lignes visibles proposés : 1 à 10 (F-03). */
const LINES = Array.from({ length: 10 }, (_, i) => i + 1);
/** Classes d'un champ (libellé au-dessus). */
const fieldClass = 'flex flex-col gap-1 text-sm';
/** Classes d'une liste déroulante ou d'un champ numérique. */
const controlClass =
  'h-9 rounded-md border border-border bg-background px-2 outline-none focus-visible:ring-2 focus-visible:ring-ring';

/** Panneau « Entraînement » de la page Réglages (F-01, F-03 à F-06). */
export default function PracticeSettingsPanel() {
  const { t } = useTranslation('practice');
  const id = useId();
  const [settings, setSettings] = useState<PracticeSettings | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Chargement des réglages ; `active` ignore une réponse arrivée après le démontage.
  useEffect(() => {
    let active = true;
    unwrap(commands.getPracticeSettings())
      .then((s) => active && setSettings(s))
      .catch((e: unknown) => active && setError(String((e as { message?: string }).message ?? e)));
    return () => {
      active = false;
    };
  }, []);

  // Erreur, puis chargement en cours, avant d'afficher le formulaire.
  if (error) {
    return (
      <p role="alert" className="text-destructive">
        {t('common:error.generic', { message: error })}
      </p>
    );
  }
  if (!settings) return <p>{t('common:loading')}</p>;

  // Mise à jour immédiate à l'écran, puis enregistrement ; le backend renvoie la valeur bornée.
  const update = (patch: Partial<PracticeSettings>) => {
    const next = { ...settings, ...patch };
    setSettings(next);
    unwrap(commands.savePracticeSettings(next))
      .then(setSettings)
      .catch((e: unknown) => setError(String((e as { message?: string }).message ?? e)));
  };

  // Type de session courant et raccourci pour le changer.
  const goalKind = settings.goal.type;
  const setGoal = (goal: SessionGoal) => update({ goal });

  return (
    <div className="grid gap-4 sm:grid-cols-2">
      {/* Taille de police (F-01). */}
      <label className={fieldClass}>
        {t('settings.fontSize', { size: settings.fontSize })}
        <input
          type="range"
          min={14}
          max={32}
          step={1}
          value={settings.fontSize}
          onChange={(e) => update({ fontSize: Number(e.target.value) })}
          className="accent-primary"
        />
      </label>
      {/* Nombre de lignes visibles (F-03). */}
      <label className={fieldClass}>
        {t('settings.visibleLines')}
        <select
          className={controlClass}
          value={settings.visibleLines}
          onChange={(e) => update({ visibleLines: Number(e.target.value) })}
        >
          {LINES.map((n) => (
            <option key={n} value={n}>
              {n}
            </option>
          ))}
        </select>
      </label>
      {/* Position de la ligne active (F-03). */}
      <fieldset className={fieldClass}>
        <legend className="mb-1">{t('settings.activeLine')}</legend>
        {(['top', 'center'] as const).map((value) => (
          <label key={value} className="flex items-center gap-2">
            <input
              type="radio"
              name={`${id}-active-line`}
              checked={settings.activeLine === value}
              onChange={() => update({ activeLine: value })}
              className="accent-primary"
            />
            {t(`settings.activeLines.${value}`)}
          </label>
        ))}
      </fieldset>
      {/* Mode de correction (F-04). */}
      <fieldset className={fieldClass}>
        <legend className="mb-1">{t('settings.correction')}</legend>
        {(['free', 'blocking'] as const).map((value) => (
          <label key={value} className="flex items-center gap-2">
            <input
              type="radio"
              name={`${id}-correction`}
              checked={settings.correction === value}
              onChange={() => update({ correction: value })}
              className="accent-primary"
            />
            {t(`settings.corrections.${value}`)}
          </label>
        ))}
      </fieldset>
      {/* Options à cocher : retour arrière, bandeau, tolérance typographique. */}
      <div className="flex flex-col gap-2 text-sm">
        {(['backspaceAllowed', 'showBanner', 'lenientPunctuation'] as const).map((key) => (
          <label key={key} className="flex items-center gap-2">
            <input
              type="checkbox"
              checked={settings[key]}
              onChange={(e) => update({ [key]: e.target.checked })}
              className="accent-primary"
            />
            {t(`settings.${key}`)}
          </label>
        ))}
      </div>
      {/* Type de session (F-06), avec son paramètre selon le type. */}
      <div className="flex flex-col gap-2">
        <label className={fieldClass}>
          {t('settings.goal')}
          <select
            className={controlClass}
            value={goalKind}
            onChange={(e) => {
              // Changer de type part d'une valeur par défaut : 1 minute ou 50 mots.
              const kind = e.target.value as SessionGoal['type'];
              setGoal(
                kind === 'duration'
                  ? { type: 'duration', minutes: 1 }
                  : kind === 'words'
                    ? { type: 'words', count: 50 }
                    : { type: 'wholeText' },
              );
            }}
          >
            {(['wholeText', 'duration', 'words'] as const).map((kind) => (
              <option key={kind} value={kind}>
                {t(`settings.goals.${kind}`)}
              </option>
            ))}
          </select>
        </label>
        {settings.goal.type === 'duration' && (
          <label className={fieldClass}>
            {t('settings.minutes')}
            <select
              className={controlClass}
              value={settings.goal.minutes}
              onChange={(e) => setGoal({ type: 'duration', minutes: Number(e.target.value) })}
            >
              {DURATIONS.map((m) => (
                <option key={m} value={m}>
                  {t('settings.minutesValue', { count: m })}
                </option>
              ))}
            </select>
          </label>
        )}
        {settings.goal.type === 'words' && (
          <label className={fieldClass}>
            {t('settings.wordCount')}
            <input
              type="number"
              min={1}
              max={10_000}
              className={controlClass}
              value={settings.goal.count}
              // Un champ vidé vaut 1 plutôt que 0.
              onChange={(e) => setGoal({ type: 'words', count: Number(e.target.value) || 1 })}
            />
          </label>
        )}
      </div>
    </div>
  );
}
