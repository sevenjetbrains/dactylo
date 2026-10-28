/** Modification d'un texte de la bibliothèque (F-23). */

import { useEffect, useState, type FormEvent } from 'react';
import { useTranslation } from 'react-i18next';

import { commands, unwrap, type TextSummary } from '@/shared/api';
import { Button } from '@/shared/ui/button';
import { Card, CardTitle } from '@/shared/ui/card';

import {
  authorOf,
  fieldClass,
  inputClass,
  messageOf,
  parseTags,
  type TextMeta,
} from './form-utils';
import { MetaFields } from './MetaFields';

/** Charge le texte `id`, le laisse modifier puis l'enregistre. */
export function TextEditor({ id, onSaved }: { id: string; onSaved: (text: TextSummary) => void }) {
  const { t } = useTranslation('library');
  const [meta, setMeta] = useState<TextMeta | null>(null);
  const [original, setOriginal] = useState('');
  const [content, setContent] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Chargement du texte ; `active` ignore une réponse arrivée après le démontage.
  useEffect(() => {
    let active = true;
    unwrap(commands.getText(id))
      .then((text) => {
        if (!active) return;
        setMeta({
          title: text.title,
          author: text.author ?? '',
          tags: text.tags.join(', '),
          language: text.language,
          segmentation: text.segmentation,
        });
        setOriginal(text.title);
        setContent(text.content);
      })
      .catch((e: unknown) => active && setError(messageOf(e)));
    return () => {
      active = false;
    };
  }, [id]);

  // Erreur de chargement, puis attente.
  if (!meta) {
    return error ? (
      <p role="alert" className="text-destructive">
        {t('common:error.generic', { message: error })}
      </p>
    ) : (
      <p>{t('common:loading')}</p>
    );
  }

  /** Enregistre les modifications. */
  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setSaving(true);
    setError(null);
    try {
      const text = await unwrap(
        commands.updateText(id, {
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
      <CardTitle>{t('edit.title', { title: original })}</CardTitle>
      <form className="flex flex-col gap-4" onSubmit={(e) => void submit(e)}>
        <MetaFields value={meta} onChange={setMeta} />
        <label className={fieldClass}>
          {t('edit.content')}
          <textarea
            className={`${inputClass} min-h-64 font-mono`}
            value={content}
            onChange={(e) => setContent(e.target.value)}
            required
          />
        </label>
        {/* Prévenir avant de perdre la progression. */}
        <p className="text-sm text-muted-foreground">{t('edit.resetWarning')}</p>
        {error && (
          <p role="alert" className="text-destructive">
            {t('common:error.generic', { message: error })}
          </p>
        )}
        <Button type="submit" disabled={saving} className="self-start">
          {t('edit.submit')}
        </Button>
      </form>
    </Card>
  );
}
