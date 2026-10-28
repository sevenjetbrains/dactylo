/** Barre de progression de la charte. */

import { cn } from '@/shared/lib/utils';

/** Barre horizontale ; `value` va de 0 à 1 et est bornée. */
export function ProgressBar({
  value,
  label,
  className,
}: {
  value: number;
  /** Nom annoncé par les lecteurs d'écran. */
  label: string;
  className?: string;
}) {
  // Bornée à [0, 1], puis convertie en pourcentage entier pour l'accessibilité.
  const percent = Math.round(Math.min(1, Math.max(0, value)) * 100);
  return (
    <div
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={percent}
      className={cn('h-1.5 w-full overflow-hidden rounded-full bg-muted', className)}
    >
      <div className="h-full rounded-full bg-primary" style={{ width: `${percent}%` }} />
    </div>
  );
}
