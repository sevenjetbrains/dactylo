/** Tests de fumée : l'application s'ouvre et se parcourt au clavier. */

import { expect, test } from '@playwright/test';

import { installFakeBackend } from './fake-backend';

// Chaque test tourne contre le faux backend IPC.
test.beforeEach(async ({ page }) => {
  await installFakeBackend(page);
});

test("l'accueil s'ouvre et salue le profil par défaut", async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Bonjour Par défaut.' })).toBeVisible();
  await expect(page.getByRole('navigation')).toContainText('Accueil');
  // Carte du profil en bas du menu.
  await expect(page.getByRole('navigation')).toContainText('AZERTY · Français');
});

test('la navigation au clavier mène aux réglages et change le thème', async ({ page }) => {
  await page.goto('/');
  // Navigation sans souris : focus sur le lien puis Entrée (NF-07).
  await page.getByRole('link', { name: 'Réglages' }).focus();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('heading', { name: 'Réglages' })).toBeVisible();

  // Le thème sombre ajoute la classe `dark` à la racine.
  await page.getByRole('radio', { name: 'Sombre' }).check();
  await expect(page.locator('html')).toHaveClass(/dark/);
});
