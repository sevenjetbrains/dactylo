import type { Page } from '@playwright/test';

/**
 * Simule le pont IPC de Tauri (ADR 0007) avec un petit backend en mémoire :
 * le webview n'est pas disponible dans un navigateur. Statistiques, nettoyage PDF
 * et découpage en exercices sont volontairement simplistes ; les vrais calculs
 * sont couverts par `cargo test`.
 */
export async function installFakeBackend(page: Page) {
  await page.addInitScript(() => {
    /** Frappe brute telle qu'envoyée par l'interface. */
    type Key = { seq: number; kind: 'insert' | 'delete'; typed: string | null; tMs: number };
    /** Découpage en exercices, comme `Segmentation` côté Rust. */
    type Segmentation = { type: 'whole' } | { type: 'paragraphs' | 'lines'; count: number };
    /** Texte stocké. */
    type Text = {
      id: string;
      title: string;
      author: string | null;
      content: string;
      tags: string[];
      source: 'paste' | 'pdf' | 'builtin';
      pages: string | null;
      sourceFile: string | null;
      segmentation: Segmentation;
      createdAt: number;
      completed: number;
    };
    // Découpage en graphèmes, comme le cœur Rust.
    const segmenter = new Intl.Segmenter('fr', { granularity: 'grapheme' });
    const split = (s: string) => Array.from(segmenter.segment(s), (x) => x.segment);
    // « Base » en mémoire : textes, sessions et réglages par défaut.
    const texts: Text[] = [];
    const sessions = new Map<
      string,
      { textId: string; keys: Key[]; exercise: { index: number; count: number }; text: string }
    >();
    const settings = {
      fontSize: 20,
      visibleLines: 3,
      activeLine: 'top',
      correction: 'free',
      backspaceAllowed: true,
      showBanner: true,
      lenientPunctuation: true,
      goal: { type: 'wholeText' },
    };
    /** Compteur pour fabriquer des identifiants uniques. */
    let counter = 0;

    /** Exercices d'un texte : une ligne non vide par bloc, regroupées par `count`. */
    const exercisesOf = (t: Text): string[] => {
      if (t.segmentation.type === 'whole') return [t.content];
      const lines = t.content.split('\n').filter((l) => l.trim());
      const size = Math.max(1, t.segmentation.count);
      const out: string[] = [];
      for (let i = 0; i < lines.length; i += size) out.push(lines.slice(i, i + size).join('\n'));
      return out.length ? out : [t.content];
    };

    /** Résumé d'un texte, au format de `TextSummary`. */
    const summary = (t: Text) => {
      const count = exercisesOf(t).length;
      return {
        id: t.id,
        title: t.title,
        author: t.author,
        language: 'fr',
        tags: t.tags,
        source: t.source,
        sourceFile: t.sourceFile,
        pages: t.pages,
        length: t.content.length,
        createdAt: t.createdAt,
        segmentation: t.segmentation,
        exerciseCount: count,
        completed: Math.min(t.completed, count),
        nextExercise: t.completed >= count ? 0 : t.completed,
        bestWpm: null,
      };
    };

    /** Ajoute un texte et renvoie son résumé. */
    const add = (input: Partial<Text> & { title: string; content: string }) => {
      const text: Text = {
        id: `t${++counter}`,
        author: null,
        tags: [],
        source: 'paste',
        pages: null,
        sourceFile: null,
        segmentation: { type: 'whole' },
        createdAt: Date.now() + counter,
        completed: 0,
        ...input,
      };
      texts.push(text);
      return summary(text);
    };

    /** Une fonction par commande Tauri, nommée comme côté Rust. */
    const handlers: Record<string, (args: Record<string, unknown>) => unknown> = {
      app_info: () => ({ name: 'Dactylo', version: '0.1.0' }),
      current_profile: () => ({
        id: 'p1',
        name: 'Par défaut',
        keyboardLayout: 'azerty',
        createdAt: 0,
      }),
      pending_session: () => null,
      get_practice_settings: () => settings,
      save_practice_settings: (a) => a.settings,
      // Du plus récent au plus ancien, filtré par titre, comme la vraie bibliothèque.
      list_texts: (a) => {
        const query = a.query as { search: string };
        return [...texts]
          .reverse()
          .filter((t) => t.title.toLowerCase().includes(query.search.toLowerCase()))
          .map(summary);
      },
      list_tags: () => [...new Set(texts.flatMap((t) => t.tags))].sort(),
      create_text: (a) => add(a.input as Text),
      get_text: (a) => {
        const text = texts.find((t) => t.id === a.id);
        if (!text) throw { code: 'notFound', message: 'texte' };
        return { ...text, language: 'fr' };
      },
      update_text: (a) => {
        const text = texts.find((t) => t.id === a.id);
        if (!text) throw { code: 'notFound', message: 'texte' };
        Object.assign(text, a.input);
        return summary(text);
      },
      delete_text: (a) => {
        texts.splice(
          texts.findIndex((t) => t.id === a.id),
          1,
        );
        return null;
      },
      duplicate_text: (a) => {
        const text = texts.find((t) => t.id === a.id);
        if (!text) throw { code: 'notFound', message: 'texte' };
        return add({ ...text, title: `${text.title} (copie)`, completed: 0 });
      },
      // Nettoyage simplifié : premières lignes (en-têtes) et numéros de page retirés,
      // césures recollées ; une ligne par paragraphe.
      preview_pdf: (a) => {
        const source = a.source as {
          pages: { number: number; lines: string[] }[];
          range: { first: number; last: number };
          lines: { first: number; last: number } | null;
        };
        const lines = source.pages
          .filter((p) => p.number >= source.range.first && p.number <= source.range.last)
          .flatMap((p) =>
            p.lines
              .slice(1)
              .flatMap((text) => (/^\d+$/.test(text) ? [] : [{ page: p.number, text }])),
          );
        if (!lines.length) throw { code: 'invalid', message: 'aucun texte trouvé (PDF scanné)' };
        const chosen = source.lines
          ? lines.slice(source.lines.first - 1, source.lines.last)
          : lines;
        const content = chosen
          .map((l) => l.text)
          .join('\n')
          .replace(/-\n(?=\p{Ll})/gu, '')
          .replace(/([^.])\n/g, '$1 ');
        return { lines, content };
      },
      import_pdf: (a) => {
        const input = a.input as Text & {
          fileName: string;
          range: { first: number; last: number };
        };
        return add({
          ...input,
          source: 'pdf',
          sourceFile: input.fileName,
          pages: `${input.range.first}-${input.range.last}`,
        });
      },
      start_session: (a) => {
        // Texte demandé, sinon le plus récent ; exercice demandé, sinon le prochain.
        const text = texts.find((t) => t.id === a.textId) ?? texts.at(-1);
        if (!text) throw { code: 'notFound', message: 'aucun texte dans la bibliothèque' };
        const all = exercisesOf(text);
        const index =
          (a.exercise as number | null) ?? (text.completed >= all.length ? 0 : text.completed);
        const content = all[index] ?? '';
        const sessionId = `s${++counter}`;
        const exercise = { index, count: all.length };
        sessions.set(sessionId, { textId: text.id, keys: [], exercise, text: content });
        const graphemes = split(content);
        // Mots : suites de lettres ou chiffres (version simplifiée de `word_spans`).
        const words: { start: number; end: number }[] = [];
        graphemes.forEach((g, i) => {
          const isWord = /\p{L}|\p{N}/u.test(g);
          const last = words.at(-1);
          if (isWord && last && last.end === i) last.end = i + 1;
          else if (isWord) words.push({ start: i, end: i + 1 });
        });
        return {
          sessionId,
          textId: text.id,
          title: text.title,
          exercise,
          graphemes,
          words,
          settings,
          keystrokes: [],
          activeMs: 0,
        };
      },
      // Une seule ligne : la mise en page n'est pas testée ici.
      wrap_text: () => [0],
      record_keystrokes: (a) => {
        const keys = a.keystrokes as Key[];
        sessions.get(a.sessionId as string)?.keys.push(...keys);
        return keys.at(-1)?.seq ?? null;
      },
      finish_session: (a) => {
        const session = sessions.get(a.sessionId as string);
        if (!session) throw { code: 'notFound', message: 'session' };
        const text = split(session.text);
        // Statistiques simplistes : une insertion est fausse si elle diffère du texte à son rang.
        const inserts = session.keys.filter((k) => k.kind === 'insert');
        const wrong = inserts.filter((k, i) => k.typed !== text[i]).length;
        const minutes = Math.max(Number(a.activeMs), 1) / 60_000;
        // Exercice fini : la progression du texte avance.
        const stored = texts.find((t) => t.id === session.textId);
        if (stored && inserts.length >= text.length) {
          stored.completed = Math.max(stored.completed, session.exercise.index + 1);
        }
        const { index, count } = session.exercise;
        return {
          sessionId: a.sessionId,
          textId: session.textId,
          exercise: session.exercise,
          nextExercise: index + 1 < count ? index + 1 : null,
          stats: {
            durationMs: a.activeMs,
            typed: inserts.length,
            errors: wrong,
            uncorrected: wrong,
            wpmRaw: inserts.length / 5 / minutes,
            wpmNet: inserts.length / 5 / minutes,
            cpm: (inserts.length - wrong) / minutes,
            accuracy: (100 * (inserts.length - wrong)) / Math.max(inserts.length, 1),
          },
          // Faute codée en dur pour le scénario « la forêt ».
          errors: wrong
            ? [{ expected: 'forêt', typed: 'foret', errorType: 'accent', position: 3, word: true }]
            : [],
          average: null,
        };
      },
      abandon_session: (a) => {
        sessions.delete(a.sessionId as string);
        return null;
      },
      next_text_id: () => null,
    };

    // Remplace le pont IPC que Tauri injecte normalement dans la WebView.
    Object.assign(window, {
      __TAURI_INTERNALS__: {
        invoke: async (cmd: string, args: Record<string, unknown> = {}) => {
          const handler = handlers[cmd];
          if (!handler) throw { code: 'notFound', message: `commande simulée absente : ${cmd}` };
          return handler(args);
        },
        transformCallback: () => 0,
      },
    });
  });
}
