/** Tests de la pagination : hook de découpage et navigation. */

import { render, renderHook, act, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';
import { describe, expect, it } from 'vitest';

import { pageSlots, usePagination } from '@/shared/lib/use-pagination';

import { Pagination } from './pagination';

/** Liste de 1 à `n`. */
const range = (n: number) => Array.from({ length: n }, (_, i) => i + 1);

/** Liste paginée par 3, avec sa navigation, comme dans les pages. */
function Harness({ items }: { items: number[] }) {
  const { page, pageCount, pageItems, setPage } = usePagination(items, 3);
  return (
    <>
      <p>{pageItems.join(',')}</p>
      <Pagination page={page} pageCount={pageCount} onPage={setPage} />
    </>
  );
}

describe('usePagination', () => {
  it('découpe la liste et borne la page demandée', () => {
    const { result } = renderHook(() => usePagination(range(7), 3));
    expect(result.current.pageCount).toBe(3);
    expect(result.current.pageItems).toEqual([1, 2, 3]);

    // Au-delà de la dernière page : on reste sur la dernière.
    act(() => result.current.setPage(9));
    expect(result.current.page).toBe(3);
    expect(result.current.pageItems).toEqual([7]);
  });

  it('reste sur une page valide quand la liste raccourcit', () => {
    const { result, rerender } = renderHook(({ items }) => usePagination(items, 3), {
      initialProps: { items: range(7) },
    });
    act(() => result.current.setPage(3));
    // Le dernier élément disparaît (suppression) : retour à la page 2.
    rerender({ items: range(6) });
    expect(result.current.page).toBe(2);
    expect(result.current.pageItems).toEqual([4, 5, 6]);
  });

  it('compte une page pour une liste vide', () => {
    const { result } = renderHook(() => usePagination([], 3));
    expect(result.current).toMatchObject({ page: 1, pageCount: 1, pageItems: [] });
  });
});

describe('pageSlots', () => {
  it('montre toutes les pages quand elles sont peu nombreuses', () => {
    expect(pageSlots(1, 5)).toEqual([1, 2, 3, 4, 5]);
  });

  it('remplace les pages lointaines par « … »', () => {
    expect(pageSlots(5, 12)).toEqual([1, 'gap', 4, 5, 6, 'gap', 12]);
    expect(pageSlots(1, 12)).toEqual([1, 2, 'gap', 12]);
    expect(pageSlots(12, 12)).toEqual([1, 'gap', 11, 12]);
  });

  it('affiche la page plutôt qu’un « … » qui n’en cacherait qu’une', () => {
    expect(pageSlots(4, 12)).toEqual([1, 2, 3, 4, 5, 'gap', 12]);
  });
});

describe('Pagination', () => {
  it('navigue entre les pages et désactive les bornes', async () => {
    render(<Harness items={range(7)} />);
    const previous = screen.getByRole('button', { name: 'Page précédente' });
    const next = screen.getByRole('button', { name: 'Page suivante' });
    expect(screen.getByText('Page 1 sur 3')).toBeInTheDocument();
    expect(previous).toBeDisabled();

    // Deux pages plus loin : dernière page, bouton « suivante » désactivé.
    await userEvent.click(next);
    await userEvent.click(next);
    expect(screen.getByText('7')).toBeInTheDocument();
    expect(screen.getByText('Page 3 sur 3')).toBeInTheDocument();
    expect(next).toBeDisabled();

    await userEvent.click(previous);
    expect(screen.getByText('4,5,6')).toBeInTheDocument();
  });

  it('va directement à la page cliquée et marque la page courante', async () => {
    render(<Harness items={range(30)} />);
    await userEvent.click(screen.getByRole('button', { name: 'Aller à la page 10' }));
    expect(screen.getByText('28,29,30')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Aller à la page 10' })).toHaveAttribute(
      'aria-current',
      'page',
    );
    // Les pages 2 à 8 sont repliées dans un « … ».
    expect(screen.queryByRole('button', { name: 'Aller à la page 5' })).toBeNull();
    expect(screen.getByText('…')).toBeInTheDocument();
  });

  it('ne s’affiche pas pour une seule page', () => {
    render(<Harness items={range(3)} />);
    expect(screen.queryByRole('navigation')).toBeNull();
  });

  it('suit la page fixée par le parent', async () => {
    /** Parent qui pilote la page lui-même. */
    function Controlled() {
      const [page, setPage] = useState(2);
      return <Pagination page={page} pageCount={4} onPage={setPage} />;
    }
    render(<Controlled />);
    await userEvent.click(screen.getByRole('button', { name: 'Page suivante' }));
    expect(screen.getByText('Page 3 sur 4')).toBeInTheDocument();
  });
});
