/** Bouton de la charte (convention shadcn/ui). */

import type { VariantProps } from 'class-variance-authority';
import type { ButtonHTMLAttributes } from 'react';

import { cn } from '@/shared/lib/utils';

import { buttonVariants } from './button-variants';

/** Propriétés d'un bouton HTML, plus variante et taille. */
export type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> &
  VariantProps<typeof buttonVariants>;

/** Bouton ; `type="button"` par défaut pour ne pas soumettre un formulaire par erreur. */
export function Button({ className, variant, size, type = 'button', ...props }: ButtonProps) {
  return (
    <button type={type} className={cn(buttonVariants({ variant, size }), className)} {...props} />
  );
}
