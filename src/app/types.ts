/** Types du registre de modules. */

import type { ComponentType, LazyExoticComponent } from 'react';
import type { LucideIcon } from 'lucide-react';

/** Composant de page, chargé tout de suite ou à la demande. */
type Page = LazyExoticComponent<ComponentType> | ComponentType;

/** Traductions d'un module, par langue ; chargées dans l'espace de noms `id` du module. */
export type ModuleLocales = Record<string, Record<string, unknown>>;

/** Déclaration d'un module auprès du registre (ADR 0005). */
export interface ModuleManifest {
  /** Identifiant unique, aussi utilisé comme espace de noms i18n. */
  id: string;
  /** Route du module. */
  route: {
    /** Chemin relatif à la racine ; '' pour l'accueil. */
    path: string;
    /** Page affichée. */
    component: Page;
  };
  /** Entrée du menu latéral ; `labelKey` est une clé de l'espace de noms du module. */
  menu?: {
    /** Clé du libellé. */
    labelKey: string;
    /** Icône lucide. */
    icon: LucideIcon;
    /** Rang dans le menu, croissant. */
    order: number;
  };
  /** Panneau ajouté à la page Réglages. */
  settingsPanel?: {
    /** Clé du titre du panneau. */
    titleKey: string;
    /** Contenu du panneau. */
    component: Page;
  };
  /** Traductions du module. */
  locales?: ModuleLocales;
}
