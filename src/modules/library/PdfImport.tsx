/**
 * Import d'un PDF (F-21, F-22) : ouverture, aperçu et choix des pages, choix des
 * lignes, options de nettoyage, puis texte nettoyé modifiable avant enregistrement.
 */

import { ChevronLeft, ChevronRight } from 'lucide-react';
import { useEffect, useRef, useState, type FormEvent } from 'react';
import { useTranslation } from 'react-i18next';

import {
  commands,
  unwrap,
  type CleanOptions_Deserialize as CleanOptions,
  type PdfPage,
  type SourceLine,
  type TextSummary,
} from '@/shared/api';
import { cn } from '@/shared/lib/utils';
import { Button } from '@/shared/ui/button';
import { Card, CardTitle } from '@/shared/ui/card';

import { authorOf, emptyMeta, fieldClass, inputClass, messageOf, parseTags } from './form-utils';
import { MetaFields } from './MetaFields';
import { openPdf, type PdfDocument } from './pdf';

/** Pages voisines extraites de chaque côté : elles aident à repérer les en-têtes répétés. */
const NEIGHBOURS = 2;
/** Largeur de l'aperçu de page, en px. */
const PREVIEW_WIDTH = 320;
/** Options de nettoyage, toutes actives par défaut (comme côté cœur). */
const OPTION_KEYS = [
  'removePageNumbers',
  'removeHeaders',
  'joinHyphens',
  'reflow',
  'normalizeSpacing',
] as const;

/** Pages extraites et plage demandée. */
interface Extracted {
  /** Pages extraites, voisines comprises. */
  pages: PdfPage[];
  /** Première page retenue. */
  first: number;
  /** Dernière page retenue. */
  last: number;
}

/** Import d'un PDF ; `onSaved` reçoit le texte enregistré. */
export function PdfImport({ onSaved }: { onSaved: (text: TextSummary) => void }) {
  const { t } = useTranslation('library');
  const [doc, setDoc] = useState<PdfDocument | null>(null);
  const [fileName, setFileName] = useState('');
  const [busy, setBusy] = useState<'opening' | 'extracting' | 'saving' | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Étape 1 : page affichée et plage choisie.
  const [shown, setShown] = useState(1);
  const [firstPage, setFirstPage] = useState(1);
  const [lastPage, setLastPage] = useState(1);
  const [extracted, setExtracted] = useState<Extracted | null>(null);
  // Étapes 2 et 3 : lignes, sélection et options.
  const [lines, setLines] = useState<SourceLine[]>([]);
  const [firstLine, setFirstLine] = useState(1);
  const [lastLine, setLastLine] = useState<number | null>(null);
  const [options, setOptions] = useState<Required<CleanOptions>>({
    removePageNumbers: true,
    removeHeaders: true,
    joinHyphens: true,
    reflow: true,
    normalizeSpacing: true,
  });
  // Étape 4 : texte nettoyé (modifiable) et métadonnées.
  const [content, setContent] = useState('');
  const [meta, setMeta] = useState(emptyMeta);
  const canvasRef = useRef<HTMLCanvasElement>(null);

  // Le document est libéré quand il est remplacé ou que l'import est quitté.
  useEffect(() => () => void doc?.close(), [doc]);

  // Aperçu de la page affichée.
  useEffect(() => {
    if (!doc || !canvasRef.current) return;
    doc.render(shown, canvasRef.current, PREVIEW_WIDTH).catch(() => undefined);
  }, [doc, shown]);

  // Aperçu nettoyé, recalculé quand la plage, les lignes ou les options changent.
  // Les champs vidés pendant la saisie (valeur 0) attendent une valeur complète.
  useEffect(() => {
    if (!extracted || firstLine < 1 || (lastLine !== null && lastLine < firstLine)) return;
    let active = true;
    unwrap(
      commands.previewPdf({
        fileName,
        pages: extracted.pages,
        range: { first: extracted.first, last: extracted.last },
        lines: lastLine === null ? null : { first: firstLine, last: lastLine },
        options,
      }),
    )
      .then((preview) => {
        if (!active) return;
        setLines(preview.lines);
        setContent(preview.content);
        setError(null);
      })
      .catch((e: unknown) => active && setError(messageOf(e)));
    return () => {
      active = false;
    };
  }, [extracted, fileName, firstLine, lastLine, options]);

  /** Ouvre le fichier choisi ; le titre proposé est son nom sans extension. */
  const open = async (file: File) => {
    setBusy('opening');
    setError(null);
    setExtracted(null);
    try {
      const opened = await openPdf(await file.arrayBuffer());
      setDoc(opened);
      setFileName(file.name);
      setMeta({ ...emptyMeta, title: file.name.replace(/\.pdf$/i, '') });
      setShown(1);
      setFirstPage(1);
      setLastPage(opened.pageCount);
    } catch {
      setError(t('pdf.invalid'));
    } finally {
      setBusy(null);
    }
  };

  /** Extrait les lignes des pages choisies, plus quelques voisines. */
  const extract = async () => {
    if (!doc) return;
    // Plage remise dans l'ordre et dans les limites du document (champ vide = page 1).
    const clamp = (n: number) => Math.min(doc.pageCount, Math.max(1, n));
    const first = clamp(Math.min(firstPage, lastPage));
    const last = clamp(Math.max(firstPage, lastPage));
    setBusy('extracting');
    try {
      const pages = await doc.extract(
        Math.max(1, first - NEIGHBOURS),
        Math.min(doc.pageCount, last + NEIGHBOURS),
      );
      // Nouvelle plage : toutes ses lignes sont retenues au départ.
      setFirstLine(1);
      setLastLine(null);
      setExtracted({ pages, first, last });
    } catch (e) {
      setError(messageOf(e));
    } finally {
      setBusy(null);
    }
  };

  /** Enregistre le texte tel qu'il apparaît dans l'aperçu (éventuellement corrigé). */
  const save = async (e: FormEvent) => {
    e.preventDefault();
    if (!extracted) return;
    setBusy('saving');
    try {
      const text = await unwrap(
        commands.importPdf({
          title: meta.title,
          author: authorOf(meta),
          tags: parseTags(meta.tags),
          language: meta.language,
          segmentation: meta.segmentation,
          fileName,
          range: { first: extracted.first, last: extracted.last },
          content,
        }),
      );
      onSaved(text);
    } catch (err) {
      setError(messageOf(err));
    } finally {
      setBusy(null);
    }
  };

  // Dernière ligne effective : toutes les lignes tant qu'aucune n'est choisie.
  const last = lastLine ?? lines.length;
  const selected = (n: number) => n >= firstLine && n <= last;

  return (
    <Card className="flex flex-col gap-6">
      <CardTitle>{t('pdf.title')}</CardTitle>

      {/* Choix du fichier. */}
      <label className={fieldClass}>
        {t('pdf.file')}
        <input
          type="file"
          accept="application/pdf,.pdf"
          className={inputClass}
          onChange={(e) => {
            const file = e.target.files?.[0];
            if (file) void open(file);
          }}
        />
      </label>
      {busy === 'opening' && <p>{t('pdf.opening')}</p>}

      {doc && (
        // Étape 1 : aperçu d'une page et plage de pages.
        <section className="flex flex-col gap-3" aria-labelledby="pdf-pages-step">
          <h3 id="pdf-pages-step" className="font-semibold">
            {t('pdf.pagesStep')}{' '}
            <span className="font-normal text-muted-foreground">
              ({t('pdf.pageCount', { count: doc.pageCount })})
            </span>
          </h3>
          <div className="flex flex-wrap items-start gap-6">
            <figure className="flex flex-col items-center gap-2">
              <canvas
                ref={canvasRef}
                aria-label={t('pdf.preview', { page: shown })}
                className="rounded-lg border border-border bg-white"
                style={{ width: PREVIEW_WIDTH }}
              />
              {/* Navigation dans l'aperçu, page par page. */}
              <figcaption className="flex items-center gap-2 text-sm">
                <Button
                  variant="outline"
                  size="sm"
                  aria-label={t('pdf.previousPage')}
                  disabled={shown <= 1}
                  onClick={() => setShown(shown - 1)}
                >
                  <ChevronLeft aria-hidden className="size-4" />
                </Button>
                <span className="tabular-nums">
                  {shown} / {doc.pageCount}
                </span>
                <Button
                  variant="outline"
                  size="sm"
                  aria-label={t('pdf.nextPage')}
                  disabled={shown >= doc.pageCount}
                  onClick={() => setShown(shown + 1)}
                >
                  <ChevronRight aria-hidden className="size-4" />
                </Button>
              </figcaption>
            </figure>
            <div className="flex flex-col gap-3">
              <div className="flex items-end gap-3">
                <label className={fieldClass}>
                  {t('pdf.firstPage')}
                  <input
                    type="number"
                    min={1}
                    max={doc.pageCount}
                    className={`${inputClass} w-24`}
                    value={firstPage}
                    onChange={(e) => setFirstPage(Number(e.target.value))}
                  />
                </label>
                <label className={fieldClass}>
                  {t('pdf.lastPage')}
                  <input
                    type="number"
                    min={1}
                    max={doc.pageCount}
                    className={`${inputClass} w-24`}
                    value={lastPage}
                    onChange={(e) => setLastPage(Number(e.target.value))}
                  />
                </label>
              </div>
              <Button
                onClick={() => void extract()}
                disabled={busy !== null}
                className="self-start"
              >
                {t('pdf.extract')}
              </Button>
              {busy === 'extracting' && <p>{t('pdf.extracting')}</p>}
            </div>
          </div>
        </section>
      )}

      {error && (
        <p role="alert" className="text-destructive">
          {t('common:error.generic', { message: error })}
        </p>
      )}

      {extracted && lines.length > 0 && (
        <>
          {/* Étape 2 : lignes de la plage, numérotées ; les lignes retenues sont en clair. */}
          <section className="flex flex-col gap-3" aria-labelledby="pdf-lines-step">
            <h3 id="pdf-lines-step" className="font-semibold">
              {t('pdf.linesStep')}
            </h3>
            <p className="text-sm text-muted-foreground">{t('pdf.linesHint')}</p>
            <div className="flex flex-wrap items-end gap-3">
              <label className={fieldClass}>
                {t('pdf.firstLine')}
                <input
                  type="number"
                  min={1}
                  max={lines.length}
                  className={`${inputClass} w-24`}
                  value={firstLine}
                  onChange={(e) => setFirstLine(Number(e.target.value))}
                />
              </label>
              <label className={fieldClass}>
                {t('pdf.lastLine')}
                <input
                  type="number"
                  min={1}
                  max={lines.length}
                  className={`${inputClass} w-24`}
                  value={last}
                  onChange={(e) => setLastLine(Number(e.target.value))}
                />
              </label>
              {/* Nombre de lignes : fixe la dernière ligne à partir de la première. */}
              <label className={fieldClass}>
                {t('pdf.lineCount')}
                <input
                  type="number"
                  min={1}
                  max={lines.length}
                  className={`${inputClass} w-24`}
                  value={last - firstLine + 1}
                  onChange={(e) => setLastLine(firstLine + Number(e.target.value) - 1)}
                />
              </label>
            </div>
            <ol className="max-h-64 overflow-auto rounded-lg border border-border bg-background p-2 font-mono text-sm">
              {lines.map((line, i) => (
                <li
                  key={i}
                  aria-label={t('pdf.lineLabel', { line: i + 1, page: line.page })}
                  className={cn(
                    'flex gap-3 px-2 py-0.5',
                    selected(i + 1) ? 'text-foreground' : 'text-muted-foreground opacity-50',
                  )}
                >
                  <span className="w-8 shrink-0 text-right tabular-nums">{i + 1}</span>
                  <span>{line.text}</span>
                </li>
              ))}
            </ol>
          </section>

          {/* Étape 3 : options de nettoyage (F-22). */}
          <fieldset className="flex flex-col gap-2 text-sm">
            <legend className="mb-2 font-semibold">{t('pdf.cleanStep')}</legend>
            {OPTION_KEYS.map((key) => (
              <label key={key} className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={options[key]}
                  onChange={(e) => setOptions({ ...options, [key]: e.target.checked })}
                  className="accent-primary"
                />
                {t(`pdf.options.${key}`)}
              </label>
            ))}
          </fieldset>

          {/* Étape 4 : texte nettoyé modifiable, métadonnées, enregistrement. */}
          <form className="flex flex-col gap-4" onSubmit={(e) => void save(e)}>
            <h3 className="font-semibold">{t('pdf.saveStep')}</h3>
            <label className={fieldClass}>
              {t('pdf.content')}
              <textarea
                className={`${inputClass} min-h-64 font-mono`}
                value={content}
                onChange={(e) => setContent(e.target.value)}
                aria-describedby="pdf-content-hint"
                required
              />
            </label>
            <p id="pdf-content-hint" className="text-xs text-muted-foreground">
              {t('pdf.contentHint')}
            </p>
            <MetaFields value={meta} onChange={setMeta} />
            <Button type="submit" disabled={busy !== null} className="self-start">
              {t('pdf.submit')}
            </Button>
          </form>
        </>
      )}
    </Card>
  );
}
