/** Barre d'outils de la session : type de session, correction, clavier, pause et recommencer. */

import { Keyboard, Pause, Play, RotateCcw } from 'lucide-react';
import { useId } from 'react';
import { useTranslation } from 'react-i18next';

import type { PracticeSettings_Serialize as PracticeSettings, SessionGoal } from '@/shared/api';
import { cn } from '@/shared/lib/utils';
import { Button } from '@/shared/ui/button';

/** Durées proposées en onglets, en minutes (F-06). */
const DURATIONS = [1, 2, 5, 10] as const;
/** Nombre de mots proposé par défaut en onglet. */
const DEFAULT_WORDS = 100;

/** Propriétés de la barre d'outils. */
interface Props {
  /** Réglages de la session en cours. */
  settings: Pick<PracticeSettings, 'goal' | 'correction' | 'showKeyboard'>;
  /** La session est-elle en pause ? */
  paused: boolean;
  /** Tout désactiver (fin de session en cours d'envoi). */
  disabled: boolean;
  /** Change le type de session. */
  onGoal: (goal: SessionGoal) => void;
  /** Change le mode de correction. */
  onCorrection: (mode: PracticeSettings['correction']) => void;
  /** Affiche ou masque le clavier virtuel, sans relancer le texte. */
  onToggleKeyboard: () => void;
  /** Met en pause ou reprend. */
  onTogglePause: () => void;
  /** Recommence le texte. */
  onRestart: () => void;
}

/** Deux objectifs sont-ils le même onglet ? */
function sameGoal(a: SessionGoal, b: SessionGoal): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/**
 * Onglets du type de session, choix de la correction et boutons pause / recommencer,
 * comme sur la maquette de la page Copie. Changer un réglage relance le texte,
 * sauf le clavier virtuel, qui n'est qu'un affichage.
 */
export function SessionToolbar({
  settings,
  paused,
  disabled,
  onGoal,
  onCorrection,
  onToggleKeyboard,
  onTogglePause,
  onRestart,
}: Props) {
  const { t } = useTranslation('practice');
  const id = useId();
  // L'onglet « mots » reprend le nombre réglé, sinon la valeur par défaut.
  const words = settings.goal.type === 'words' ? settings.goal.count : DEFAULT_WORDS;
  const tabs: { goal: SessionGoal; label: string }[] = [
    { goal: { type: 'wholeText' }, label: t('toolbar.wholeText') },
    ...DURATIONS.map((minutes) => ({
      goal: { type: 'duration', minutes } as const,
      label: t('toolbar.minutes', { count: minutes }),
    })),
    { goal: { type: 'words', count: words }, label: t('toolbar.words', { count: words }) },
  ];

  return (
    <div className="flex flex-wrap items-center gap-3">
      {/* Onglets du type de session (F-06) : l'onglet actif est sur fond blanc. */}
      <div
        role="group"
        aria-label={t('toolbar.goal')}
        className="flex rounded-xl border border-border bg-muted p-1"
      >
        {tabs.map(({ goal, label }) => {
          const selected = sameGoal(goal, settings.goal);
          return (
            <button
              key={label}
              type="button"
              aria-pressed={selected}
              disabled={disabled}
              onClick={() => !selected && onGoal(goal)}
              className={cn(
                'rounded-lg px-3.5 py-1.5 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring',
                selected
                  ? 'bg-card font-semibold shadow-xs'
                  : 'text-muted-foreground hover:text-foreground',
              )}
            >
              {label}
            </button>
          );
        })}
      </div>
      {/* Mode de correction (F-04). */}
      <label htmlFor={`${id}-correction`} className="text-sm text-muted-foreground">
        {t('toolbar.correction')}
      </label>
      <select
        id={`${id}-correction`}
        value={settings.correction}
        disabled={disabled}
        onChange={(e) => onCorrection(e.target.value as PracticeSettings['correction'])}
        className="h-10 min-w-40 rounded-lg border border-border bg-card px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
      >
        {(['free', 'blocking'] as const).map((mode) => (
          <option key={mode} value={mode}>
            {t(`corrections.${mode}`)}
          </option>
        ))}
      </select>
      {/* Clavier virtuel (F-08) : bouton enfoncé quand il est affiché. */}
      <Button
        variant="outline"
        size="icon"
        aria-pressed={settings.showKeyboard}
        onClick={onToggleKeyboard}
        aria-label={settings.showKeyboard ? t('toolbar.hideKeyboard') : t('toolbar.showKeyboard')}
        title={settings.showKeyboard ? t('toolbar.hideKeyboard') : t('toolbar.showKeyboard')}
        className={cn(settings.showKeyboard && 'bg-muted text-primary')}
      >
        <Keyboard aria-hidden className="size-4" />
      </Button>
      {/* Pause / reprise (Échap) et recommencer (Ctrl+R). */}
      <Button
        variant="outline"
        size="icon"
        disabled={disabled}
        onClick={onTogglePause}
        aria-label={paused ? t('toolbar.resume') : t('toolbar.pause')}
        title={paused ? t('toolbar.resume') : t('toolbar.pause')}
      >
        {paused ? (
          <Play aria-hidden className="size-4" />
        ) : (
          <Pause aria-hidden className="size-4" />
        )}
      </Button>
      <Button
        variant="outline"
        size="icon"
        disabled={disabled}
        onClick={onRestart}
        aria-label={t('toolbar.restart')}
        title={t('toolbar.restart')}
      >
        <RotateCcw aria-hidden className="size-4" />
      </Button>
    </div>
  );
}
