/** Configuration des tests de bout en bout Playwright. */

import { defineConfig, devices } from '@playwright/test';

// e2e sur le serveur Vite avec l'IPC Tauri simulé (ADR 0007).
export default defineConfig({
  testDir: 'tests/e2e',
  fullyParallel: true,
  // En CI : un `test.only` oublié fait échouer, et un test instable a droit à un nouvel essai.
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  use: { baseURL: 'http://localhost:1420', trace: 'on-first-retry' },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  // Lance Vite avant les tests ; en local, réutilise un serveur déjà démarré.
  webServer: {
    command: 'pnpm dev',
    url: 'http://localhost:1420',
    reuseExistingServer: !process.env.CI,
  },
});
