/** Page Bibliothèque. */

import { ClipboardPaste, Copy, FileText, Pencil, Trash2 } from 'lucide-react';
import { useCallback, useEffect, useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useSearchParams } from 'react-router';

import { commands, unwrap, type TextSort, type TextSummary } from '@/shared/api';
import { cn } from '@/shared/lib/utils';
import { Button } from '@/shared/ui/button';
import { buttonVariants } from '@/shared/ui/button-variants';
import { Eyebrow } from '@/shared/ui/card';
import { ProgressBar } from '@/shared/ui/progress-bar';

import { fieldClass, inputClass, messageOf } from './form-utils';
import { PasteForm } from './PasteForm';
import { PdfImport } from './PdfImport';
import { TextEditor } from './TextEditor';

/** Ordres de tri proposés. */
const SORTS: TextSort[] = ['newest', 'oldest', 'shortest', 'longest', 'bestScore'];

/**
 * Bibliothèque (F-20 à F-25). La vue se choisit par l'URL : liste par défaut,
 * `?view=paste`, `?view=pdf` ou `?view=edit&id=<id>` ; l'accueil y renvoie aussi.
 */
export default function LibraryPage() {
  const { t } = useTranslation('library');
  const [params, setParams] = useSearchParams();
  const view = params.get('view');
  const editId = params.get('id');
  // Message de confirmation affiché en revenant à la liste.
  const [notice, setNotice] = useState<string | null>(null);

  /** Retour à la liste avec un message. */
  const backToList = useCallback(
    (message: string) => {
      setNotice(message);
      setParams({});
    },
    [setParams],
  );

  return (
    <div className="mx-auto flex max-w-5xl flex-col gap-6">
      {/* En-tête : titre et accès aux deux manières d'ajouter un texte. */}
      <header className="flex flex-wrap items-end justify-between gap-4">
        <h1 className="text-3xl font-bold tracking-tight">{t('title')}</h1>
        <div className="flex flex-wrap gap-3">
          {view && (
            <Link to="/library" className={buttonVariants({ variant: 'ghost' })}>
              {t('actions.back')}
            </Link>
          )}
          <Link to="/library?view=paste" className={buttonVariants({ variant: 'outline' })}>
            <ClipboardPaste aria-hidden className="size-4" />
            {t('actions.paste')}
          </Link>
          <Link to="/library?view=pdf" className={buttonVariants({ variant: 'outline' })}>
            <FileText aria-hidden className="size-4" />
            {t('actions.pdf')}
          </Link>
        </div>
      </header>

      {view === 'paste' && (
        <PasteForm onSaved={(text) => backToList(t('paste.saved', { title: text.title }))} />
      )}
      {view === 'pdf' && (
        <PdfImport onSaved={(text) => backToList(t('paste.saved', { title: text.title }))} />
      )}
      {view === 'edit' && editId && (
        <TextEditor
          id={editId}
          onSaved={(text) => backToList(t('edit.saved', { title: text.title }))}
        />
      )}
      {!view && <TextList notice={notice} onNotice={setNotice} />}
    </div>
  );
}

/** Liste des textes avec recherche, filtre par étiquette et tri (F-23). */
function TextList({
  notice,
  onNotice,
}: {
  notice: string | null;
  onNotice: (message: string) => void;
}) {
  const { t } = useTranslation('library');
  const id = useId();
  const [search, setSearch] = useState('');
  const [tag, setTag] = useState('');
  const [sort, setSort] = useState<TextSort>('newest');
  const [texts, setTexts] = useState<TextSummary[] | null>(null);
  const [tags, setTags] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  // Incrémenté après une duplication ou une suppression pour recharger la liste.
  const [revision, setRevision] = useState(0);

  // Liste recalculée par le backend à chaque changement de critère.
  useEffect(() => {
    let active = true;
    unwrap(commands.listTexts({ search, tag: tag || null, sort }))
      .then((list) => active && setTexts(list))
      .catch((e: unknown) => active && setError(messageOf(e)));
    return () => {
      active = false;
    };
  }, [search, tag, sort, revision]);

  // Étiquettes disponibles pour le filtre.
  useEffect(() => {
    let active = true;
    unwrap(commands.listTags())
      .then((list) => active && setTags(list))
      .catch(() => undefined);
    return () => {
      active = false;
    };
  }, [revision]);

  /** Exécute une action sur un texte, puis recharge la liste et affiche le message. */
  const act = async (action: Promise<unknown>, message: string) => {
    try {
      await action;
      onNotice(message);
      setRevision((r) => r + 1);
    } catch (e) {
      setError(messageOf(e));
    }
  };

  return (
    <section aria-labelledby={`${id}-list`} className="flex flex-col gap-4">
      <h2 id={`${id}-list`} className="sr-only">
        {t('list.title')}
      </h2>
      {/* Message de confirmation, annoncé par les lecteurs d'écran. */}
      {notice && (
        <p role="status" className="text-sm text-muted-foreground">
          {notice}
        </p>
      )}
      {/* Critères : recherche, étiquette, tri. */}
      <div className="flex flex-wrap items-end gap-4">
        <label className={cn(fieldClass, 'min-w-60 flex-1')}>
          {t('filters.search')}
          <input
            type="search"
            className={inputClass}
            placeholder={t('filters.searchPlaceholder')}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </label>
        <label className={fieldClass}>
          {t('filters.tag')}
          <select
            className={`${inputClass} h-10`}
            value={tag}
            onChange={(e) => setTag(e.target.value)}
          >
            <option value="">{t('filters.allTags')}</option>
            {tags.map((value) => (
              <option key={value} value={value}>
                {value}
              </option>
            ))}
          </select>
        </label>
        <label className={fieldClass}>
          {t('filters.sort')}
          <select
            className={`${inputClass} h-10`}
            value={sort}
            onChange={(e) => setSort(e.target.value as TextSort)}
          >
            {SORTS.map((value) => (
              <option key={value} value={value}>
                {t(`filters.sorts.${value}`)}
              </option>
            ))}
          </select>
        </label>
      </div>

      {error && (
        <p role="alert" className="text-destructive">
          {t('common:error.generic', { message: error })}
        </p>
      )}

      {/* Chargement, liste vide (ou sans résultat), puis les textes. */}
      {texts === null ? (
        <p>{t('common:loading')}</p>
      ) : texts.length === 0 ? (
        <p className="text-muted-foreground">
          {search || tag ? t('list.noMatch') : t('list.empty')}
        </p>
      ) : (
        <ul className="flex flex-col gap-3">
          {texts.map((text) => (
            <TextItem
              key={text.id}
              text={text}
              onDuplicate={() =>
                void act(
                  unwrap(commands.duplicateText(text.id)),
                  t('list.duplicated', { title: text.title }),
                )
              }
              onDelete={() =>
                void act(
                  unwrap(commands.deleteText(text.id)),
                  t('list.deleted', { title: text.title }),
                )
              }
            />
          ))}
        </ul>
      )}
    </section>
  );
}

/** Un texte de la liste : informations, progression et actions. */
function TextItem({
  text,
  onDuplicate,
  onDelete,
}: {
  text: TextSummary;
  onDuplicate: () => void;
  onDelete: () => void;
}) {
  const { t, i18n } = useTranslation('library');
  // La suppression demande une confirmation, sur place.
  const [confirming, setConfirming] = useState(false);
  const date = new Intl.DateTimeFormat(i18n.language, { dateStyle: 'medium' }).format(
    new Date(text.createdAt),
  );
  // Provenance affichée en surtitre : pack intégré ou pages du PDF.
  const origin =
    text.source === 'builtin'
      ? t('list.builtin')
      : text.source === 'pdf' && text.pages
        ? t('list.pdfPages', { pages: text.pages })
        : null;
  const multi = text.exerciseCount > 1;

  return (
    <li className="flex flex-wrap items-center justify-between gap-4 rounded-2xl border border-border bg-card px-5 py-4 shadow-xs">
      <div className="flex min-w-60 flex-1 flex-col gap-1">
        {origin && <Eyebrow>{origin}</Eyebrow>}
        <span className="font-semibold">{text.title}</span>
        <span className="text-xs text-muted-foreground">
          {[text.author, t('list.meta', { length: text.length, date }), text.tags.join(', ')]
            .filter(Boolean)
            .join(' · ')}
        </span>
        {/* Progression dans les exercices (F-24) et meilleur score. */}
        <div className="mt-1 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs">
          {multi && (
            <>
              <span>
                {t('list.exercise', { current: text.nextExercise + 1, count: text.exerciseCount })}
              </span>
              <ProgressBar
                value={text.completed / text.exerciseCount}
                label={t('list.exercisesDone', { done: text.completed, count: text.exerciseCount })}
                className="w-32"
              />
            </>
          )}
          {text.bestWpm != null && (
            <span className="text-muted-foreground">
              {t('list.best', { wpm: Math.round(text.bestWpm) })}
            </span>
          )}
        </div>
      </div>
      {/* Actions : s'entraîner, modifier, dupliquer, supprimer (avec confirmation). */}
      <div className="flex items-center gap-2">
        {confirming ? (
          <>
            <Button variant="outline" size="sm" onClick={() => setConfirming(false)}>
              {t('list.cancel')}
            </Button>
            <Button
              size="sm"
              className="bg-destructive text-primary-foreground hover:bg-destructive/90"
              onClick={onDelete}
            >
              {t('list.confirmDelete')}
            </Button>
          </>
        ) : (
          <>
            <Link
              to={`/practice?text=${encodeURIComponent(text.id)}`}
              className={buttonVariants({ size: 'sm' })}
              aria-label={t('list.practiceLabel', { title: text.title })}
            >
              {t('list.practice')}
            </Link>
            {/* Dictée du même texte (F-10). */}
            <Link
              to={`/dictation?text=${encodeURIComponent(text.id)}`}
              className={buttonVariants({ variant: 'outline', size: 'sm' })}
              aria-label={t('list.dictationLabel', { title: text.title })}
            >
              {t('list.dictation')}
            </Link>
            <Link
              to={`/library?view=edit&id=${encodeURIComponent(text.id)}`}
              className={buttonVariants({ variant: 'ghost', size: 'sm' })}
              aria-label={t('list.edit', { title: text.title })}
              title={t('list.edit', { title: text.title })}
            >
              <Pencil aria-hidden className="size-4" />
            </Link>
            <Button
              variant="ghost"
              size="sm"
              aria-label={t('list.duplicate', { title: text.title })}
              title={t('list.duplicate', { title: text.title })}
              onClick={onDuplicate}
            >
              <Copy aria-hidden className="size-4" />
            </Button>
            <Button
              variant="ghost"
              size="sm"
              aria-label={t('list.delete', { title: text.title })}
              title={t('list.delete', { title: text.title })}
              onClick={() => setConfirming(true)}
            >
              <Trash2 aria-hidden className="size-4" />
            </Button>
          </>
        )}
      </div>
    </li>
  );
}
