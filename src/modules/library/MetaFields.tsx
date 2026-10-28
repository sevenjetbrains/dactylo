/** Champs communs d'un texte : titre, auteur, étiquettes, langue et découpage (F-20, F-24). */

import { useId } from 'react';
import { useTranslation } from 'react-i18next';

import type { Segmentation } from '@/shared/api';

import { fieldClass, inputClass, type TextMeta } from './form-utils';

/** Langues proposées pour un texte. */
const LANGUAGES = ['fr', 'en'] as const;

/** Champ du découpage en exercices : type, puis nombre par exercice (F-24). */
export function SegmentationField({
  value,
  onChange,
}: {
  value: Segmentation;
  onChange: (value: Segmentation) => void;
}) {
  const { t } = useTranslation('library');
  const id = useId();
  // Nombre par exercice courant ; 1 paragraphe ou 10 lignes en passant d'un type à l'autre.
  const count = value.type === 'whole' ? null : value.count;
  return (
    <div className="grid gap-4 sm:grid-cols-2">
      <label className={fieldClass}>
        {t('segmentation.label')}
        <select
          className={`${inputClass} h-10`}
          value={value.type}
          onChange={(e) => {
            const type = e.target.value as Segmentation['type'];
            onChange(
              type === 'whole' ? { type } : { type, count: count ?? (type === 'lines' ? 10 : 1) },
            );
          }}
        >
          {(['whole', 'paragraphs', 'lines'] as const).map((type) => (
            <option key={type} value={type}>
              {t(`segmentation.${type}`)}
            </option>
          ))}
        </select>
      </label>
      {/* Le nombre n'a de sens que pour un découpage par paragraphes ou par lignes. */}
      {value.type !== 'whole' && (
        <label className={fieldClass} htmlFor={`${id}-count`}>
          {t('segmentation.count')}
          <input
            id={`${id}-count`}
            type="number"
            min={1}
            max={100}
            className={inputClass}
            value={value.count}
            // Un champ vidé pendant la saisie vaut 0 ; le backend le ramène à 1.
            onChange={(e) => onChange({ ...value, count: Number(e.target.value) })}
          />
        </label>
      )}
    </div>
  );
}

/** Titre, auteur, étiquettes, langue et découpage d'un texte. */
export function MetaFields({
  value,
  onChange,
}: {
  value: TextMeta;
  onChange: (value: TextMeta) => void;
}) {
  const { t } = useTranslation('library');
  const id = useId();
  // Change un seul champ en gardant les autres.
  const set = <K extends keyof TextMeta>(key: K, v: TextMeta[K]) =>
    onChange({ ...value, [key]: v });
  return (
    <div className="flex flex-col gap-4">
      <div className="grid gap-4 sm:grid-cols-2">
        <label className={fieldClass}>
          {t('paste.textTitle')}
          <input
            className={inputClass}
            value={value.title}
            onChange={(e) => set('title', e.target.value)}
            required
            maxLength={200}
          />
        </label>
        <label className={fieldClass}>
          {t('paste.author')}
          <input
            className={inputClass}
            value={value.author}
            onChange={(e) => set('author', e.target.value)}
          />
        </label>
      </div>
      <div className="grid gap-4 sm:grid-cols-2">
        {/* Étiquettes, avec une aide reliée au champ. */}
        <div className={fieldClass}>
          <label htmlFor={`${id}-tags`}>{t('paste.tags')}</label>
          <input
            id={`${id}-tags`}
            className={inputClass}
            value={value.tags}
            onChange={(e) => set('tags', e.target.value)}
            aria-describedby={`${id}-tags-hint`}
          />
          <span id={`${id}-tags-hint`} className="text-xs text-muted-foreground">
            {t('paste.tagsHint')}
          </span>
        </div>
        <label className={fieldClass}>
          {t('paste.language')}
          <select
            className={`${inputClass} h-10`}
            value={value.language}
            onChange={(e) => set('language', e.target.value)}
          >
            {LANGUAGES.map((l) => (
              <option key={l} value={l}>
                {t(`languages.${l}`)}
              </option>
            ))}
          </select>
        </label>
      </div>
      <SegmentationField value={value.segmentation} onChange={(s) => set('segmentation', s)} />
    </div>
  );
}
