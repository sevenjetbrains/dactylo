/** Configuration des tests unitaires, construite sur celle de Vite. */

import { defineConfig, mergeConfig } from 'vitest/config';

import viteConfig from './vite.config.ts';

export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      environment: 'jsdom',
      // Le pool « forks » expire au démarrage sous Windows ; les threads sont fiables.
      pool: 'threads',
      globals: true,
      setupFiles: ['./src/test/setup.ts'],
      include: ['src/**/*.test.{ts,tsx}'],
      // Le client généré est exclu de la couverture.
      coverage: { provider: 'v8', include: ['src/**'], exclude: ['src/shared/api/bindings.ts'] },
    },
  }),
);
