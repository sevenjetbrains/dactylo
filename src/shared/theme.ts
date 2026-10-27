import { useEffect } from 'react';
import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export type ThemePreference = 'light' | 'dark' | 'system';

interface ThemeState {
  theme: ThemePreference;
  setTheme: (theme: ThemePreference) => void;
}

// Préférence d'affichage locale ; les réglages par profil arrivent avec F-51.
export const useThemeStore = create<ThemeState>()(
  persist((set) => ({ theme: 'system', setTheme: (theme) => set({ theme }) }), {
    name: 'dactylo-theme',
  }),
);

function prefersDark(): boolean {
  return window.matchMedia?.('(prefers-color-scheme: dark)').matches ?? false;
}

/** Applique la classe `dark` sur <html> selon la préférence et le système. */
export function useApplyTheme(): void {
  const theme = useThemeStore((s) => s.theme);

  useEffect(() => {
    const apply = () => {
      const dark = theme === 'dark' || (theme === 'system' && prefersDark());
      document.documentElement.classList.toggle('dark', dark);
    };
    apply();
    if (theme !== 'system' || !window.matchMedia) return;
    const query = window.matchMedia('(prefers-color-scheme: dark)');
    query.addEventListener('change', apply);
    return () => query.removeEventListener('change', apply);
  }, [theme]);
}
