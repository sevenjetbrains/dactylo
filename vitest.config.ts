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
      coverage: { provider: 'v8', include: ['src/**'], exclude: ['src/shared/api/bindings.ts'] },
    },
  }),
);
