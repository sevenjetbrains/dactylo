/** Colonne « Réglages » de la dictée : profil, voix, vitesse, pause, découpage,
 * avancement, ponctuation lue (F-11 à F-15, F-17). */

import { useEffect, useId, useState } from 'react';
import { useTranslation } from 'react-i18next';

import {
  commands,
  unwrap,
  type DictationProfile_Serialize as DictationProfile,
  type DictationSettings_Serialize as DictationSettings,
  type SegmentMode,
  type Voice,
} from '@/shared/api';
import { formatNumber } from '@/shared/lib/format';
import { cn } from '@/shared/lib/utils';
import { Card } from '@/shared/ui/card';

/** Découpages proposés, dans l'ordre de la maquette. */
const MODES: SegmentMode[] = ['word', 'group', 'sentence', 'paragraph'];

/** Classes d'une liste déroulante. */
const selectClass =
  'h-10 w-full rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring';

/** Propriétés de la colonne. */
interface Props {
  /** Réglages en vigueur. */
  settings: DictationSettings;
  /** Le découpage peut-il encore changer (aucun segment lu) ? */
  segmentationLocked: boolean;
  /** Nouveaux réglages choisis. */
  onChange: (settings: DictationSettings) => void;
}

/** Voix triées : françaises d'abord, puis par nom. */
function sortVoices(voices: Voice[]): Voice[] {
  const fr = (v: Voice) => (v.language.toLowerCase().startsWith('fr') ? 0 : 1);
  return [...voices].sort((a, b) => fr(a) - fr(b) || a.name.localeCompare(b.name));
}

/** Colonne des réglages, appliqués au segment suivant. */
export function DictationSidebar({ settings, segmentationLocked, onChange }: Props) {
  const { t, i18n } = useTranslation('dictation');
  const lang = i18n.language;
  const id = useId();
  const [voices, setVoices] = useState<Voice[]>([]);
  const [profiles, setProfiles] = useState<DictationProfile[]>([]);
  const [profileId, setProfileId] = useState('');

  // Voix installées et profils enregistrés ; une erreur laisse simplement la liste vide.
  useEffect(() => {
    let active = true;
    unwrap(commands.listVoices())
      .then((v) => active && setVoices(sortVoices(v)))
      .catch(() => undefined);
    unwrap(commands.listDictationProfiles())
      .then((p) => active && setProfiles(p))
      .catch(() => undefined);
    return () => {
      active = false;
    };
  }, []);

  /** Applique un changement partiel des réglages. */
  const patch = (p: Partial<DictationSettings>) => onChange({ ...settings, ...p });
  const rate = settings.rate ?? 1;
  const pauseS = settings.pauseMs / 1000;

  return (
    <Card className="flex flex-col gap-5">
      {/* En-tête : titre et profil de dictée (F-17). */}
      <div className="flex items-center justify-between gap-3">
        <h2 className="text-lg font-semibold">{t('sidebar.title')}</h2>
        <select
          aria-label={t('sidebar.profile')}
          className={cn(selectClass, 'w-auto max-w-[10rem]')}
          value={profileId}
          onChange={(e) => {
            setProfileId(e.target.value);
            const chosen = profiles.find((p) => p.id === e.target.value);
            if (chosen) onChange(chosen.settings);
          }}
        >
          <option value="">{t('sidebar.customProfile')}</option>
          {profiles.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
      </div>

      {/* Voix (F-12). */}
      <label htmlFor={`${id}-voice`} className="flex flex-col gap-1 text-sm">
        {t('sidebar.voice')}
        <select
          id={`${id}-voice`}
          className={selectClass}
          value={settings.voice ?? ''}
          onChange={(e) => patch({ voice: e.target.value || null })}
        >
          <option value="">{t('sidebar.defaultVoice')}</option>
          {voices.map((v) => (
            <option key={v.id} value={v.id}>
              {v.name} — {v.language}
            </option>
          ))}
        </select>
      </label>

      {/* Vitesse, de 0,5× à 2× (F-12). */}
      <label htmlFor={`${id}-rate`} className="flex flex-col gap-1 text-sm">
        <span className="flex justify-between">
          {t('sidebar.rate')}
          <span className="font-mono text-primary">
            {t('sidebar.rateValue', { value: formatNumber(rate, lang, 1) })}
          </span>
        </span>
        <input
          id={`${id}-rate`}
          type="range"
          min={0.5}
          max={2}
          step={0.1}
          value={rate}
          onChange={(e) => patch({ rate: Number(e.target.value) })}
          className="accent-primary"
        />
      </label>

      {/* Pause entre segments, de 0 à 10 s (F-12). */}
      <label htmlFor={`${id}-pause`} className="flex flex-col gap-1 text-sm">
        <span className="flex justify-between">
          {t('sidebar.pause')}
          <span className="font-mono text-primary">
            {t('sidebar.pauseValue', { value: formatNumber(pauseS, lang, pauseS % 1 ? 1 : 0) })}
          </span>
        </span>
        <input
          id={`${id}-pause`}
          type="range"
          min={0}
          max={10}
          step={0.5}
          value={pauseS}
          onChange={(e) => patch({ pauseMs: Math.round(Number(e.target.value) * 1000) })}
          className="accent-primary"
        />
      </label>

      {/* Découpage (F-11) : figé une fois la lecture commencée. */}
      <fieldset className="flex flex-col gap-2 text-sm">
        <legend className="mb-2">{t('sidebar.segmentation')}</legend>
        <div className="grid grid-cols-2 gap-2">
          {MODES.map((m) => (
            <button
              key={m}
              type="button"
              aria-pressed={settings.segmentation === m}
              disabled={segmentationLocked}
              onClick={() => patch({ segmentation: m })}
              className={cn(
                'h-10 rounded-lg border text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-60',
                settings.segmentation === m
                  ? 'border-primary bg-primary/5 font-semibold text-primary'
                  : 'border-border bg-card hover:bg-muted',
              )}
            >
              {m === 'group'
                ? t('sidebar.group', { count: settings.groupSize })
                : t(`sidebar.modes.${m}`)}
            </button>
          ))}
        </div>
        {segmentationLocked && (
          <p className="text-xs text-muted-foreground">{t('sidebar.segmentationLocked')}</p>
        )}
      </fieldset>

      {/* Avancement (F-15). */}
      <fieldset className="flex flex-col gap-2 text-sm">
        <legend className="mb-2">{t('sidebar.advance')}</legend>
        {(['adaptive', 'automatic'] as const).map((a) => (
          <label key={a} className="flex items-center gap-2">
            <input
              type="radio"
              name={`${id}-advance`}
              checked={settings.advance === a}
              onChange={() => patch({ advance: a })}
              className="accent-primary"
            />
            {t(`sidebar.advances.${a}`)}
          </label>
        ))}
      </fieldset>

      {/* Ponctuation lue (F-13). */}
      <label className="flex items-start gap-2 border-t border-border pt-4 text-sm">
        <input
          type="checkbox"
          checked={settings.readPunctuation}
          onChange={(e) => patch({ readPunctuation: e.target.checked })}
          className="mt-1 accent-primary"
        />
        <span>
          {t('sidebar.readPunctuation')}
          <span className="block text-xs text-muted-foreground">
            {t('sidebar.readPunctuationHint')}
          </span>
        </span>
      </label>
    </Card>
  );
}
