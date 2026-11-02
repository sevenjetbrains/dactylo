/** Navigation entre les pages d'une liste. */

import { ChevronLeft, ChevronRight } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { pageSlots } from '@/shared/lib/use-pagination';
import { cn } from '@/shared/lib/utils';

import { Button } from './button';

/**
 * Boutons « précédente » / « suivante » et numéros de page cliquables.
 * Rien n'est affiché quand la liste tient sur une seule page.
 */
export function Pagination({
  page,
  pageCount,
  onPage,
  className,
}: {
  /** Page affichée, à partir de 1. */
  page: number;
  /** Nombre de pages. */
  pageCount: number;
  /** Demande l'affichage d'une autre page. */
  onPage: (page: number) => void;
  className?: string;
}) {
  const { t } = useTranslation('common');
  // Une seule page : la navigation n'apporterait que du bruit.
  if (pageCount <= 1) return null;

  return (
    <nav
      aria-label={t('pagination.label')}
      className={cn('flex flex-wrap items-center justify-center gap-1.5 text-sm', className)}
    >
      <Button
        variant="outline"
        size="sm"
        disabled={page <= 1}
        onClick={() => onPage(page - 1)}
        aria-label={t('pagination.previous')}
        title={t('pagination.previous')}
      >
        <ChevronLeft aria-hidden className="size-4" />
      </Button>
      {/* Numéros de page ; la page courante est en surbrillance et non cliquable. */}
      {pageSlots(page, pageCount).map((slot, i) =>
        slot === 'gap' ? (
          <span key={`gap-${i}`} aria-hidden className="px-1 text-muted-foreground">
            …
          </span>
        ) : (
          <Button
            key={slot}
            variant={slot === page ? 'default' : 'ghost'}
            size="sm"
            aria-current={slot === page ? 'page' : undefined}
            aria-label={t('pagination.goTo', { page: slot })}
            onClick={() => slot !== page && onPage(slot)}
            className="min-w-8 px-2 tabular-nums"
          >
            {slot}
          </Button>
        ),
      )}
      <Button
        variant="outline"
        size="sm"
        disabled={page >= pageCount}
        onClick={() => onPage(page + 1)}
        aria-label={t('pagination.next')}
        title={t('pagination.next')}
      >
        <ChevronRight aria-hidden className="size-4" />
      </Button>
      {/* Position annoncée aux lecteurs d'écran à chaque changement de page. */}
      <span aria-live="polite" className="sr-only">
        {t('pagination.position', { page, count: pageCount })}
      </span>
    </nav>
  );
}
