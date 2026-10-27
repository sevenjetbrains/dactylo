import type { HTMLAttributes } from 'react';

import { cn } from '@/shared/lib/utils';

export function Card({ className, ...props }: HTMLAttributes<HTMLElement>) {
  return (
    <section
      className={cn('rounded-lg border border-border bg-card p-6 text-card-foreground', className)}
      {...props}
    />
  );
}

export function CardTitle({ className, ...props }: HTMLAttributes<HTMLHeadingElement>) {
  return <h2 className={cn('mb-3 text-lg font-semibold', className)} {...props} />;
}
