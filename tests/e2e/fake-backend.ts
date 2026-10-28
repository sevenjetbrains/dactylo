import type { Page } from '@playwright/test';

/**
 * Simule le pont IPC de Tauri (ADR 0007) avec un petit backend en mémoire :
 * le webview n'est pas disponible dans un navigateur. Les statistiques sont
 * volontairement simplistes ; le vrai calcul est couvert par `cargo test`.
 */
export async function installFakeBackend(page: Page) {
  await page.addInitScript(() => {
    type Key = { seq: number; kind: 'insert' | 'delete'; typed: string | null; tMs: number };
    const segmenter = new Intl.Segmenter('fr', { granularity: 'grapheme' });
    const split = (s: string) => Array.from(segmenter.segment(s), (x) => x.segment);
    const texts: { id: string; title: string; content: string; createdAt: number }[] = [];
    const sessions = new Map<string, { textId: string; keys: Key[] }>();
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
    let counter = 0;

    const summary = (t: (typeof texts)[number]) => ({
      id: t.id,
      title: t.title,
      language: 'fr',
      tags: [],
      length: t.content.length,
      createdAt: t.createdAt,
    });

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
      list_texts: () => [...texts].reverse().map(summary),
      create_text: (a) => {
        const input = a.input as { title: string; content: string };
        const text = {
          id: `t${++counter}`,
          title: input.title,
          content: input.content,
          createdAt: Date.now(),
        };
        texts.push(text);
        return summary(text);
      },
      start_session: (a) => {
        const text = texts.find((t) => t.id === a.textId) ?? texts.at(-1);
        if (!text) throw { code: 'notFound', message: 'aucun texte dans la bibliothèque' };
        const sessionId = `s${++counter}`;
        sessions.set(sessionId, { textId: text.id, keys: [] });
        const graphemes = split(text.content);
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
          graphemes,
          words,
          settings,
          keystrokes: [],
          activeMs: 0,
        };
      },
      wrap_text: () => [0],
      record_keystrokes: (a) => {
        const keys = a.keystrokes as Key[];
        sessions.get(a.sessionId as string)?.keys.push(...keys);
        return keys.at(-1)?.seq ?? null;
      },
      finish_session: (a) => {
        const session = sessions.get(a.sessionId as string);
        if (!session) throw { code: 'notFound', message: 'session' };
        const text = split(texts.find((t) => t.id === session.textId)?.content ?? '');
        const inserts = session.keys.filter((k) => k.kind === 'insert');
        const wrong = inserts.filter((k, i) => k.typed !== text[i]).length;
        const minutes = Math.max(Number(a.activeMs), 1) / 60_000;
        return {
          sessionId: a.sessionId,
          textId: session.textId,
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
