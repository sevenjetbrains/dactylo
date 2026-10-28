/** Formulaire d'ajout d'un texte par copier-coller (F-20). */

import { useState, type FormEvent } from 'react';
import { useTranslation } from 'react-i18next';

import { commands, unwrap, type TextSummary } from '@/shared/api';
import { Button } from '@/shared/ui/button';
import { Card, CardTitle } from '@/shared/ui/card';

import { authorOf, emptyMeta, fieldClass, inputClass, messageOf, parseTags } from './form-utils';
import { MetaFields } from './MetaFields';

/** Collage d'un texte ; `onSaved` reçoit le texte enregistré. */
export function PasteForm({ onSaved }: { onSaved: (text: TextSummary) => void }) {
  const { t } = useTranslation('library');
  const [meta, setMeta] = useState(emptyMeta);
  const [content, setContent] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  /** Enregistre le texte ; le backend le normalise et refuse un texte vide. */
  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setSaving(true);
    setError(null);
    try {
      const text = await unwrap(
        commands.createText({
          title: meta.title,
          author: authorOf(meta),
          content,
          tags: parseTags(meta.tags),
          language: meta.language,
          segmentation: meta.segmentation,
        }),
      );
      onSaved(text);
    } catch (err) {
      setError(messageOf(err));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Card>
      <CardTitle>{t('paste.title')}</CardTitle>
      <form className="flex flex-col gap-4" onSubmit={(e) => void submit(e)}>
        <MetaFields value={meta} onChange={setMeta} />
        <label className={fieldClass}>
          {t('paste.content')}
          <textarea
            className={`${inputClass} min-h-48 font-mono`}
            value={content}
            onChange={(e) => setContent(e.target.value)}
            required
          />
        </label>
        {error && (
          <p role="alert" className="text-destructive">
            {t('common:error.generic', { message: error })}
          </p>
        )}
        <Button type="submit" disabled={saving} className="self-start">
          {t('paste.submit')}
        </Button>
      </form>
    </Card>
  );
}
