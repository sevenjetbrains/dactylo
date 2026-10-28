import { expect, test } from '@playwright/test';

import { installFakeBackend } from './fake-backend';

test.beforeEach(async ({ page }) => {
  await installFakeBackend(page);
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
