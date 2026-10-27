import { expect, test } from '@playwright/test';

// Simule le pont IPC de Tauri : le webview n'est pas disponible dans un navigateur.
test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    const responses: Record<string, unknown> = {
      app_info: { name: 'Dactylo', version: '0.1.0' },
      current_profile: { id: 'p1', name: 'Par défaut', keyboardLayout: 'azerty', createdAt: 0 },
    };
    Object.assign(window, {
      __TAURI_INTERNALS__: {
        invoke: async (cmd: string) => responses[cmd],
        transformCallback: () => 0,
      },
    });
  });
});

test("l'accueil s'ouvre et salue le profil par défaut", async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Bienvenue, Par défaut' })).toBeVisible();
  await expect(page.getByRole('navigation')).toContainText('Accueil');
});

test('la navigation au clavier mène aux réglages et change le thème', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('link', { name: 'Réglages' }).focus();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('heading', { name: 'Réglages' })).toBeVisible();

  await page.getByRole('radio', { name: 'Sombre' }).check();
  await expect(page.locator('html')).toHaveClass(/dark/);
});
