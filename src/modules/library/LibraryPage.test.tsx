import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import LibraryPage from './LibraryPage';

const listTexts = vi.fn();
const createText = vi.fn();

vi.mock('@/shared/api', () => ({
  commands: {
    listTexts: () => listTexts(),
    createText: (input: unknown) => createText(input),
  },
  unwrap: async (p: Promise<{ status: string; data?: unknown; error?: unknown }>) => {
    const r = await p;
    if (r.status === 'error') throw r.error;
    return r.data;
  },
}));

const loup = {
  id: 't1',
  title: 'Le loup',
  language: 'fr',
  tags: ['conte'],
  length: 18,
  createdAt: Date.UTC(2026, 9, 28),
};

function renderLibrary() {
  return render(
    <MemoryRouter>
      <LibraryPage />
    </MemoryRouter>,
  );
}

describe('LibraryPage', () => {
  beforeEach(() => {
    listTexts.mockReset();
    createText.mockReset();
  });

  it('indique une bibliothèque vide', async () => {
    listTexts.mockResolvedValue({ status: 'ok', data: [] });
    renderLibrary();
    expect(await screen.findByText('La bibliothèque est vide.')).toBeInTheDocument();
  });

  it('ajoute un texte collé avec ses étiquettes (F-20)', async () => {
    listTexts
      .mockResolvedValueOnce({ status: 'ok', data: [] })
      .mockResolvedValueOnce({ status: 'ok', data: [loup] });
    createText.mockResolvedValue({ status: 'ok', data: loup });
    renderLibrary();
    const user = userEvent.setup();

    await user.type(screen.getByLabelText('Titre'), 'Le loup');
    await user.type(screen.getByLabelText('Texte (collez-le ici)'), 'Il était une fois.');
    await user.type(screen.getByLabelText('Étiquettes'), 'conte, , loup ');
    await user.click(screen.getByRole('button', { name: 'Enregistrer' }));

    expect(createText).toHaveBeenCalledWith({
      title: 'Le loup',
      content: 'Il était une fois.',
      tags: ['conte', 'loup'],
      language: 'fr',
    });
    expect(await screen.findByRole('status')).toHaveTextContent('« Le loup » a été ajouté.');
    expect(screen.getByRole('link', { name: "S'entraîner sur « Le loup »" })).toHaveAttribute(
      'href',
      '/practice?text=t1',
    );
    expect(screen.getByLabelText('Titre')).toHaveValue('');
  });

  it('affiche le refus du backend', async () => {
    listTexts.mockResolvedValue({ status: 'ok', data: [] });
    createText.mockResolvedValue({
      status: 'error',
      error: { code: 'invalid', message: 'le texte est vide' },
    });
    renderLibrary();
    const user = userEvent.setup();
    await user.type(screen.getByLabelText('Titre'), 'X');
    await user.type(screen.getByLabelText('Texte (collez-le ici)'), ' ');
    await user.click(screen.getByRole('button', { name: 'Enregistrer' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('le texte est vide');
  });
});
