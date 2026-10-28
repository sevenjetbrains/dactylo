/** Page Bibliothèque. */

import { useCallback, useEffect, useId, useState, type FormEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router';

import { commands, unwrap, type TextSummary } from '@/shared/api';
import { Button } from '@/shared/ui/button';
import { Card, CardTitle } from '@/shared/ui/card';

/** Langues proposées pour un texte. */
const LANGUAGES = ['fr', 'en'] as const;
/** Classes d'un champ de saisie. */
const inputClass =
  'rounded-md border border-border bg-background px-3 py-2 outline-none focus-visible:ring-2 focus-visible:ring-ring';

/** Message lisible d'une erreur, qu'elle vienne du backend (`ApiError`) ou d'ailleurs. */
function messageOf(e: unknown): string {
  return typeof e === 'object' && e !== null && 'message' in e ? String(e.message) : String(e);
}

/** Étiquettes saisies séparées par des virgules. */
function parseTags(raw: string): string[] {
  return raw
    .split(',')
    .map((t) => t.trim())
    .filter(Boolean);
}

/** Bibliothèque : ajout par copier-coller (F-20) et liste des textes. */
export default function LibraryPage() {
  const { t, i18n } = useTranslation('library');
  const id = useId();
  // Liste des textes (`null` pendant le chargement), erreur et état du formulaire.
  const [texts, setTexts] = useState<TextSummary[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [title, setTitle] = useState('');
  const [content, setContent] = useState('');
  const [tags, setTags] = useState('');
  const [language, setLanguage] = useState<string>('fr');
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState<string | null>(null);

  /** Recharge la liste après un ajout. */
  const refresh = useCallback(
    () =>
      unwrap(commands.listTexts())
        .then(setTexts)
        .catch((e: unknown) => setError(messageOf(e))),
    [],
  );

  // Chargement initial ; `active` ignore une réponse arrivée après le démontage.
  useEffect(() => {
    let active = true;
    unwrap(commands.listTexts())
      .then((list) => active && setTexts(list))
      .catch((e: unknown) => active && setError(messageOf(e)));
    return () => {
      active = false;
    };
  }, []);

  /** Enregistre le texte collé, vide le formulaire et confirme l'ajout. */
  const submit = async (e: FormEvent) => {
    // Pas de rechargement de page ; on efface les messages précédents.
    e.preventDefault();
    setSaving(true);
    setError(null);
    setSaved(null);
    try {
      const text = await unwrap(
        commands.createText({ title, content, tags: parseTags(tags), language }),
      );
      // Succès : formulaire vidé, confirmation, liste rechargée.
      setTitle('');
      setContent('');
      setTags('');
      setSaved(text.title);
      await refresh();
    } catch (err) {
      setError(messageOf(err));
    } finally {
      setSaving(false);
    }
  };

  // Dates affichées selon la langue de l'interface.
  const dateFormat = new Intl.DateTimeFormat(i18n.language, { dateStyle: 'medium' });

  return (
    <div className="mx-auto flex max-w-3xl flex-col gap-6">
      <h1 className="text-2xl font-semibold">{t('title')}</h1>

      {/* Formulaire de collage (F-20). */}
      <Card>
        <CardTitle>{t('paste.title')}</CardTitle>
        <form className="flex flex-col gap-4" onSubmit={(e) => void submit(e)}>
          <label className="flex flex-col gap-1 text-sm">
            {t('paste.textTitle')}
            <input
              className={inputClass}
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              required
              maxLength={200}
            />
          </label>
          <label className="flex flex-col gap-1 text-sm">
            {t('paste.content')}
            <textarea
              className={`${inputClass} min-h-40 font-mono`}
              value={content}
              onChange={(e) => setContent(e.target.value)}
              required
            />
          </label>
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="flex flex-col gap-1 text-sm">
              <label htmlFor={`${id}-tags`}>{t('paste.tags')}</label>
              <input
                id={`${id}-tags`}
                className={inputClass}
                value={tags}
                onChange={(e) => setTags(e.target.value)}
                aria-describedby={`${id}-tags-hint`}
              />
              <span id={`${id}-tags-hint`} className="text-xs text-muted-foreground">
                {t('paste.tagsHint')}
              </span>
            </div>
            <label className="flex flex-col gap-1 text-sm">
              {t('paste.language')}
              <select
                className={`${inputClass} h-10`}
                value={language}
                onChange={(e) => setLanguage(e.target.value)}
              >
                {LANGUAGES.map((l) => (
                  <option key={l} value={l}>
                    {t(`languages.${l}`)}
                  </option>
                ))}
              </select>
            </label>
          </div>
          <div className="flex items-center gap-4">
            <Button type="submit" disabled={saving}>
              {t('paste.submit')}
            </Button>
            {/* Confirmation annoncée par les lecteurs d'écran. */}
            {saved && (
              <p role="status" className="text-sm text-muted-foreground">
                {t('paste.saved', { title: saved })}
              </p>
            )}
          </div>
        </form>
      </Card>

      {error && (
        <p role="alert" className="text-destructive">
          {t('common:error.generic', { message: error })}
        </p>
      )}

      {/* Liste des textes, du plus récent au plus ancien, avec un lien d'entraînement. */}
      <section aria-labelledby={`${id}-list`} className="flex flex-col gap-3">
        <h2 id={`${id}-list`} className="text-lg font-semibold">
          {t('list.title')}
        </h2>
        {texts === null ? (
          <p>{t('common:loading')}</p>
        ) : texts.length === 0 ? (
          <p className="text-muted-foreground">{t('list.empty')}</p>
        ) : (
          <ul className="flex flex-col gap-2">
            {texts.map((text) => (
              <li
                key={text.id}
                className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-border bg-card px-4 py-3"
              >
                <div className="flex flex-col">
                  <span className="font-medium">{text.title}</span>
                  <span className="text-xs text-muted-foreground">
                    {t('list.meta', {
                      length: text.length,
                      date: dateFormat.format(new Date(text.createdAt)),
                    })}
                    {text.tags.length > 0 && ` · ${text.tags.join(', ')}`}
                  </span>
                </div>
                <Link
                  to={`/practice?text=${encodeURIComponent(text.id)}`}
                  className="inline-flex h-8 items-center rounded-md bg-primary px-3 text-sm font-medium text-primary-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  aria-label={t('list.practiceLabel', { title: text.title })}
                >
                  {t('list.practice')}
                </Link>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
