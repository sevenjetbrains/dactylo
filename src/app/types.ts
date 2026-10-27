import type { ComponentType, LazyExoticComponent } from 'react';
import type { LucideIcon } from 'lucide-react';

type Page = LazyExoticComponent<ComponentType> | ComponentType;

/** Traductions d'un module, par langue ; chargées dans l'espace de noms `id` du module. */
export type ModuleLocales = Record<string, Record<string, unknown>>;

/** Déclaration d'un module auprès du registre (ADR 0005). */
export interface ModuleManifest {
  /** Identifiant unique, aussi utilisé comme espace de noms i18n. */
  id: string;
  route: {
    /** Chemin relatif à la racine ; '' pour l'accueil. */
    path: string;
    component: Page;
  };
  /** Entrée du menu latéral ; `labelKey` est une clé de l'espace de noms du module. */
  menu?: {
    labelKey: string;
    icon: LucideIcon;
    order: number;
  };
  /** Panneau ajouté à la page Réglages. */
  settingsPanel?: {
    titleKey: string;
    component: Page;
  };
  locales?: ModuleLocales;
}
