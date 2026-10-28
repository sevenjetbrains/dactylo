/** Tests de la page Bibliothèque : liste, collage, modification, import PDF. */

import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { TextSummary } from '@/shared/api';

import LibraryPage from './LibraryPage';

/** Espions des commandes utilisées par la bibliothèque. */
const api = {
  listTexts: vi.fn(),
  listTags: vi.fn(),
  createText: vi.fn(),
  getText: vi.fn(),
  updateText: vi.fn(),
  deleteText: vi.fn(),
  duplicateText: vi.fn(),
  previewPdf: vi.fn(),
  importPdf: vi.fn(),
};

// Toute commande appelée est redirigée vers l'espion du même nom ; `unwrap` reproduit
// le vrai comportement (rejet en cas d'erreur).
vi.mock('@/shared/api', () => ({
  commands: new Proxy(
    {},
    {
      get:
        (_, name: string) =>
        (...args: unknown[]) =>
          api[name as keyof typeof api](...args),
    },
  ),
  unwrap: async (p: Promise<{ status: string; data?: unknown; error?: unknown }>) => {
    const r = await p;
    if (r.status === 'error') throw r.error;
    return r.data;
  },
}));

/** Document PDF simulé : six pages ; pdf.js ne tourne pas sous jsdom. */
const pdfDoc = {
  pageCount: 6,
  extract: vi.fn(async (first: number, last: number) =>
    Array.from({ length: last - first + 1 }, (_, i) => ({
      number: first + i,
      lines: [`Ligne de la page ${first + i}`],
    })),
  ),
  render: vi.fn(async () => {}),
  close: vi.fn(async () => {}),
};

vi.mock('./pdf', () => ({ openPdf: vi.fn(async () => pdfDoc) }));

/** Réponse réussie au format tauri-specta. */
const ok = (data: unknown) => Promise.resolve({ status: 'ok', data });

/** Résumé de texte, modifiable champ par champ. */
function summary(overrides: Partial<TextSummary> = {}): TextSummary {
  return {
    id: 't1',
    title: 'Le loup',
    author: null,
    language: 'fr',
    tags: ['conte'],
    source: 'paste',
    sourceFile: null,
    pages: null,
    length: 18,
    createdAt: Date.UTC(2026, 9, 28),
    segmentation: { type: 'whole' },
    exerciseCount: 1,
    completed: 0,
    nextExercise: 0,
    bestWpm: null,
    ...overrides,
  };
}

/** Rend la bibliothèque à l'URL donnée. */
function renderLibrary(entry = '/library') {
  return render(
    <MemoryRouter initialEntries={[entry]}>
      <LibraryPage />
    </MemoryRouter>,
  );
}

describe('LibraryPage — liste (F-23)', () => {
  // Par défaut : bibliothèque vide, aucune étiquette.
  beforeEach(() => {
    for (const fn of Object.values(api)) fn.mockReset();
    api.listTexts.mockImplementation(() => ok([]));
    api.listTags.mockImplementation(() => ok([]));
  });

  it('indique une bibliothèque vide', async () => {
    renderLibrary();
    expect(await screen.findByText('La bibliothèque est vide.')).toBeInTheDocument();
  });

  it('affiche provenance, progression des exercices et meilleur score', async () => {
    api.listTexts.mockImplementation(() =>
      ok([
        summary({
          title: 'La Parure',
          author: 'Guy de Maupassant',
          source: 'builtin',
          exerciseCount: 8,
          completed: 2,
          nextExercise: 2,
          bestWpm: 41.6,
        }),
      ]),
    );
    renderLibrary();
    expect(await screen.findByText('Pack intégré')).toBeInTheDocument();
    expect(screen.getByText('Exercice 3 sur 8')).toBeInTheDocument();
    expect(screen.getByRole('progressbar', { name: '2 / 8 exercices faits' })).toHaveAttribute(
      'aria-valuenow',
      '25',
    );
    expect(screen.getByText('Meilleur : 42 MPM')).toBeInTheDocument();
    expect(screen.getByText(/Guy de Maupassant/)).toBeInTheDocument();
  });

  it('transmet recherche, étiquette et tri au backend', async () => {
    api.listTags.mockImplementation(() => ok(['conte', 'fable']));
    renderLibrary();
    const user = userEvent.setup();
    await user.type(await screen.findByLabelText('Rechercher'), 'loup');
    await user.selectOptions(screen.getByLabelText('Étiquette'), 'fable');
    await user.selectOptions(screen.getByLabelText('Trier par'), 'bestScore');
    expect(api.listTexts).toHaveBeenLastCalledWith({
      search: 'loup',
      tag: 'fable',
      sort: 'bestScore',
    });
    // Sans résultat avec des critères : message adapté.
    expect(await screen.findByText('Aucun texte ne correspond.')).toBeInTheDocument();
  });

  it('duplique, puis supprime après confirmation', async () => {
    api.listTexts.mockImplementation(() => ok([summary()]));
    api.duplicateText.mockImplementation(() => ok(summary({ id: 't2' })));
    api.deleteText.mockImplementation(() => ok(null));
    renderLibrary();
    const user = userEvent.setup();
    // Duplication : commande appelée, liste rechargée, message.
    await user.click(await screen.findByRole('button', { name: 'Dupliquer « Le loup »' }));
    expect(api.duplicateText).toHaveBeenCalledWith('t1');
    expect(await screen.findByRole('status')).toHaveTextContent('« Le loup » a été dupliqué.');
    // Suppression : rien tant que la confirmation n'est pas donnée.
    await user.click(screen.getByRole('button', { name: 'Supprimer « Le loup »' }));
    expect(api.deleteText).not.toHaveBeenCalled();
    await user.click(screen.getByRole('button', { name: 'Confirmer la suppression' }));
    expect(api.deleteText).toHaveBeenCalledWith('t1');
    await waitFor(() =>
      expect(screen.getByRole('status')).toHaveTextContent('« Le loup » a été supprimé.'),
    );
  });

  it('propose de s’entraîner et de modifier chaque texte', async () => {
    api.listTexts.mockImplementation(() => ok([summary()]));
    renderLibrary();
    expect(
      await screen.findByRole('link', { name: "S'entraîner sur « Le loup »" }),
    ).toHaveAttribute('href', '/practice?text=t1');
    expect(screen.getByRole('link', { name: 'Modifier « Le loup »' })).toHaveAttribute(
      'href',
      '/library?view=edit&id=t1',
    );
  });
});

describe('LibraryPage — collage et modification', () => {
  beforeEach(() => {
    for (const fn of Object.values(api)) fn.mockReset();
    api.listTexts.mockImplementation(() => ok([]));
    api.listTags.mockImplementation(() => ok([]));
  });

  it('ajoute un texte collé avec ses étiquettes et son découpage (F-20, F-24)', async () => {
    api.createText.mockImplementation(() => ok(summary()));
    renderLibrary('/library?view=paste');
    const user = userEvent.setup();
    // Remplissage : étiquettes avec un vide et des espaces, découpage par 2 lignes.
    await user.type(screen.getByLabelText('Titre'), 'Le loup');
    await user.type(screen.getByLabelText('Auteur (facultatif)'), 'Perrault');
    await user.type(screen.getByLabelText('Texte (collez-le ici)'), 'Il était une fois.');
    await user.type(screen.getByLabelText('Étiquettes'), 'conte, , loup ');
    await user.selectOptions(screen.getByLabelText('Découpage en exercices'), 'lines');
    await user.clear(screen.getByLabelText('Nombre par exercice'));
    await user.type(screen.getByLabelText('Nombre par exercice'), '2');
    await user.click(screen.getByRole('button', { name: 'Enregistrer' }));
    expect(api.createText).toHaveBeenCalledWith({
      title: 'Le loup',
      author: 'Perrault',
      content: 'Il était une fois.',
      tags: ['conte', 'loup'],
      language: 'fr',
      segmentation: { type: 'lines', count: 2 },
    });
    // Retour à la liste avec confirmation.
    expect(await screen.findByRole('status')).toHaveTextContent('« Le loup » a été ajouté.');
  });

  it('affiche le refus du backend', async () => {
    api.createText.mockImplementation(() =>
      Promise.resolve({
        status: 'error',
        error: { code: 'invalid', message: 'le texte est vide' },
      }),
    );
    renderLibrary('/library?view=paste');
    const user = userEvent.setup();
    await user.type(screen.getByLabelText('Titre'), 'X');
    await user.type(screen.getByLabelText('Texte (collez-le ici)'), ' ');
    await user.click(screen.getByRole('button', { name: 'Enregistrer' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('le texte est vide');
  });

  it('modifie un texte existant', async () => {
    api.getText.mockImplementation(() =>
      ok({
        id: 't1',
        title: 'Le loup',
        author: null,
        language: 'fr',
        tags: ['conte'],
        source: 'paste',
        pages: null,
        content: 'Il était une fois.',
        segmentation: { type: 'whole' },
      }),
    );
    api.updateText.mockImplementation(() => ok(summary({ title: 'Le grand loup' })));
    renderLibrary('/library?view=edit&id=t1');
    const user = userEvent.setup();
    // Le formulaire est prérempli avec le texte.
    const title = await screen.findByLabelText('Titre');
    expect(title).toHaveValue('Le loup');
    expect(screen.getByLabelText('Texte')).toHaveValue('Il était une fois.');
    await user.clear(title);
    await user.type(title, 'Le grand loup');
    await user.click(screen.getByRole('button', { name: 'Enregistrer les modifications' }));
    expect(api.updateText).toHaveBeenCalledWith('t1', {
      title: 'Le grand loup',
      author: null,
      content: 'Il était une fois.',
      tags: ['conte'],
      language: 'fr',
      segmentation: { type: 'whole' },
    });
    expect(await screen.findByRole('status')).toHaveTextContent('« Le grand loup » a été modifié.');
  });
});

describe('LibraryPage — import PDF (F-21, F-22)', () => {
  beforeEach(() => {
    for (const fn of Object.values(api)) fn.mockReset();
    pdfDoc.extract.mockClear();
    api.listTexts.mockImplementation(() => ok([]));
    api.listTags.mockImplementation(() => ok([]));
    // Aperçu : une ligne par page de la plage, texte assemblé.
    api.previewPdf.mockImplementation((source: { range: { first: number; last: number } }) =>
      ok({
        lines: [3, 4, 5]
          .filter((n) => n >= source.range.first && n <= source.range.last)
          .map((page) => ({ page, text: `Ligne de la page ${page}` })),
        content: 'Texte nettoyé.',
      }),
    );
    api.importPdf.mockImplementation(() => ok(summary({ title: 'livre', source: 'pdf' })));
  });

  /** Ouvre le PDF simulé et choisit les pages 3 à 5. */
  async function openAndExtract() {
    renderLibrary('/library?view=pdf');
    const user = userEvent.setup();
    const file = new File(['%PDF'], 'livre.pdf', { type: 'application/pdf' });
    await user.upload(screen.getByLabelText('Fichier PDF'), file);
    expect(await screen.findByText(/6 pages/)).toBeInTheDocument();
    await user.clear(screen.getByLabelText('De la page'));
    await user.type(screen.getByLabelText('De la page'), '3');
    await user.clear(screen.getByLabelText('à la page'));
    await user.type(screen.getByLabelText('à la page'), '5');
    await user.click(screen.getByRole('button', { name: 'Afficher les lignes' }));
    return user;
  }

  it('importe les pages 3 à 5 : extraction, aperçu nettoyé, enregistrement', async () => {
    const user = await openAndExtract();
    // Deux pages voisines de chaque côté sont extraites (bornées au document).
    expect(pdfDoc.extract).toHaveBeenCalledWith(1, 6);
    // Lignes numérotées avec leur page, texte nettoyé modifiable.
    expect(await screen.findByLabelText('Ligne 1, page 3')).toBeInTheDocument();
    expect(screen.getByLabelText('Texte nettoyé (modifiable)')).toHaveValue('Texte nettoyé.');
    expect(api.previewPdf).toHaveBeenLastCalledWith(
      expect.objectContaining({
        fileName: 'livre.pdf',
        range: { first: 3, last: 5 },
        lines: null,
      }),
    );
    // Titre proposé : nom du fichier ; enregistrement avec la plage et le texte.
    expect(screen.getByLabelText('Titre')).toHaveValue('livre');
    await user.click(screen.getByRole('button', { name: 'Enregistrer le texte' }));
    expect(api.importPdf).toHaveBeenCalledWith(
      expect.objectContaining({
        title: 'livre',
        fileName: 'livre.pdf',
        range: { first: 3, last: 5 },
        content: 'Texte nettoyé.',
      }),
    );
    expect(await screen.findByRole('status')).toHaveTextContent('« livre » a été ajouté.');
  });

  it('transmet le choix des lignes et les options de nettoyage', async () => {
    const user = await openAndExtract();
    await screen.findByLabelText('Ligne 1, page 3');
    // Deux lignes à partir de la deuxième.
    await user.clear(screen.getByLabelText('Première ligne'));
    await user.type(screen.getByLabelText('Première ligne'), '2');
    await user.clear(screen.getByLabelText('Nombre de lignes'));
    await user.type(screen.getByLabelText('Nombre de lignes'), '2');
    await waitFor(() =>
      expect(api.previewPdf).toHaveBeenLastCalledWith(
        expect.objectContaining({ lines: { first: 2, last: 3 } }),
      ),
    );
    // Une option décochée est transmise.
    await user.click(screen.getByRole('checkbox', { name: 'Reconstituer les paragraphes' }));
    await waitFor(() =>
      expect(api.previewPdf).toHaveBeenLastCalledWith(
        expect.objectContaining({ options: expect.objectContaining({ reflow: false }) }),
      ),
    );
  });

  it('explique qu’un PDF scanné ne contient pas de texte', async () => {
    api.previewPdf.mockImplementation(() =>
      Promise.resolve({
        status: 'error',
        error: { code: 'invalid', message: 'aucun texte trouvé (un PDF scanné…)' },
      }),
    );
    await openAndExtract();
    expect(await screen.findByRole('alert')).toHaveTextContent('PDF scanné');
  });
});
