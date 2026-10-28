/** Tests de l'écran de fin de session. */

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router';
import { describe, expect, it, vi } from 'vitest';

import type { SessionSummary_Serialize as Summary } from '@/shared/api';

import { formatDuration } from './format';
import { SessionSummary } from './SessionSummary';

/** Résumé type : une faute de mot, une de ponctuation, et une moyenne sur 10 sessions. */
const summary: Summary = {
  sessionId: 's1',
  textId: 't1',
  stats: {
    durationMs: 65_000,
    typed: 300,
    errors: 6,
    uncorrected: 2,
    wpmRaw: 55.4,
    wpmNet: 53.5,
    cpm: 270,
    accuracy: 98,
  },
  errors: [
    { expected: 'forêt', typed: 'foret', errorType: 'accent', position: 3, word: true },
    { expected: ',', typed: '.', errorType: 'punctuation', position: 8, word: false },
  ],
  average: { sessions: 10, wpmNet: 50, cpm: 280, accuracy: 98 },
};

/** Rend l'écran de fin et renvoie les deux rappels espions. */
function renderSummary(s: Summary = summary) {
  const onRestart = vi.fn();
  const onNext = vi.fn();
  render(
    <MemoryRouter>
      <SessionSummary summary={s} onRestart={onRestart} onNext={onNext} />
    </MemoryRouter>,
  );
  return { onRestart, onNext };
}

describe('SessionSummary (F-32)', () => {
  it('affiche les chiffres de la session', () => {
    renderSummary();
    expect(screen.getByText('MPM net').nextSibling).toHaveTextContent('54');
    expect(screen.getByText('Précision').nextSibling).toHaveTextContent('98 %');
    expect(screen.getByText('Durée').nextSibling).toHaveTextContent('1:05');
    expect(screen.getByText('Erreurs').nextSibling).toHaveTextContent('6');
  });

  it('compare à la moyenne des 10 dernières sessions', () => {
    renderSummary();
    expect(screen.getByText(/moyenne de vos 10 dernières sessions/)).toBeInTheDocument();
    // MPM au-dessus, CPM en dessous, précision égale.
    expect(screen.getByText('+4 par rapport à la moyenne')).toBeInTheDocument();
    expect(screen.getByText('−10 par rapport à la moyenne')).toBeInTheDocument();
    expect(screen.getByText('±0 % par rapport à la moyenne')).toBeInTheDocument();
  });

  it('liste les mots fautifs avec le type de faute, sans la ponctuation isolée', () => {
    renderSummary();
    expect(screen.getByRole('heading', { name: '1 mot fautif' })).toBeInTheDocument();
    expect(screen.getByText('forêt')).toBeInTheDocument();
    expect(screen.getByText('foret')).toBeInTheDocument();
    expect(screen.getByText('accent')).toBeInTheDocument();
  });

  it('félicite quand aucun mot n’est faux et annonce la première session', () => {
    renderSummary({ ...summary, errors: [], average: null });
    expect(screen.getByText('Bravo, aucun mot mal écrit !')).toBeInTheDocument();
    expect(screen.getByText(/Première session/)).toBeInTheDocument();
    expect(screen.queryByText(/par rapport à la moyenne/)).not.toBeInTheDocument();
  });

  it('propose de recommencer ou de passer au texte suivant', async () => {
    const { onRestart, onNext } = renderSummary();
    await userEvent.click(screen.getByRole('button', { name: 'Recommencer' }));
    await userEvent.click(screen.getByRole('button', { name: 'Texte suivant' }));
    expect(onRestart).toHaveBeenCalledOnce();
    expect(onNext).toHaveBeenCalledOnce();
  });

  it('formate les durées', () => {
    expect(formatDuration(0)).toBe('0:00');
    expect(formatDuration(9_999)).toBe('0:09');
    expect(formatDuration(600_000)).toBe('10:00');
  });
});
