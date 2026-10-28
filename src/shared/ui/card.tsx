/** Carte : conteneur encadré de la charte (convention shadcn/ui). */

import type { HTMLAttributes } from 'react';

import { cn } from '@/shared/lib/utils';

/** Carte, rendue comme une section : fond blanc, coins arrondis, bordure discrète. */
export function Card({ className, ...props }: HTMLAttributes<HTMLElement>) {
  return (
    <section
      className={cn(
        'rounded-2xl border border-border bg-card p-6 text-card-foreground shadow-xs',
        className,
      )}
      {...props}
    />
  );
}

/** Titre de carte. */
export function CardTitle({ className, ...props }: HTMLAttributes<HTMLHeadingElement>) {
  return <h2 className={cn('mb-3 text-lg font-semibold', className)} {...props} />;
}

/** Surtitre de carte ou de page : petites capitales colorées, avec icône facultative. */
export function Eyebrow({ className, ...props }: HTMLAttributes<HTMLParagraphElement>) {
  return (
    <p
      className={cn(
        'flex items-center gap-2 text-xs font-semibold tracking-wider text-primary uppercase',
        className,
      )}
      {...props}
    />
  );
}
