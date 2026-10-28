/** Thème clair / sombre : préférence mémorisée et application sur la page. */

import { useEffect } from 'react';
import { create } from 'zustand';
import { persist } from 'zustand/middleware';

/** Thème choisi ; `system` suit le réglage du système. */
export type ThemePreference = 'light' | 'dark' | 'system';

/** État du store du thème. */
interface ThemeState {
  /** Préférence courante. */
  theme: ThemePreference;
  /** Change la préférence. */
  setTheme: (theme: ThemePreference) => void;
}

// Préférence d'affichage locale ; les réglages par profil arrivent avec F-51.
export const useThemeStore = create<ThemeState>()(
  persist((set) => ({ theme: 'system', setTheme: (theme) => set({ theme }) }), {
    name: 'dactylo-theme',
  }),
);

/** Le système est-il en mode sombre ? (faux si l'API manque, comme sous jsdom) */
function prefersDark(): boolean {
  return window.matchMedia?.('(prefers-color-scheme: dark)').matches ?? false;
}

/** Applique la classe `dark` sur <html> selon la préférence et le système. */
export function useApplyTheme(): void {
  const theme = useThemeStore((s) => s.theme);

  useEffect(() => {
    // Calcule le thème effectif et met à jour la classe.
    const apply = () => {
      const dark = theme === 'dark' || (theme === 'system' && prefersDark());
      document.documentElement.classList.toggle('dark', dark);
    };
    apply();
    // En mode système, on suit les changements du système en direct.
    if (theme !== 'system' || !window.matchMedia) return;
    const query = window.matchMedia('(prefers-color-scheme: dark)');
    query.addEventListener('change', apply);
    return () => query.removeEventListener('change', apply);
  }, [theme]);
}
