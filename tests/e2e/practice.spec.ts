import { expect, test, type Page } from '@playwright/test';

import { installFakeBackend } from './fake-backend';

test.beforeEach(async ({ page }) => {
  await installFakeBackend(page);
});

async function pasteText(page: Page, title: string, content: string) {
  await page.goto('/#/library');
  await page.getByLabel('Titre').fill(title);
  await page.getByLabel('Texte (collez-le ici)').fill(content);
  await page.getByRole('button', { name: 'Enregistrer' }).click();
  await expect(page.getByRole('status')).toContainText(`« ${title} » a été ajouté.`);
}

test('coller un texte, le recopier avec une faute et voir le résumé', async ({ page }) => {
  await pasteText(page, 'La forêt', 'la forêt');
  await page.getByRole('link', { name: "S'entraîner sur « La forêt »" }).click();

  const input = page.getByLabel('Zone de frappe');
  await expect(input).toBeFocused();
  await expect(page.getByLabel('Statistiques en direct')).toBeVisible();
  // « e » à la place de « ê » : une faute d'accent.
  await page.keyboard.type('la foret');

  await expect(page.getByRole('heading', { name: 'Session terminée' })).toBeVisible();
  await expect(page.getByRole('heading', { name: '1 mot fautif' })).toBeVisible();
  await expect(page.getByText('accent', { exact: true })).toBeVisible();
});

test('Échap met en pause et Ctrl+R recommence', async ({ page }) => {
  await pasteText(page, 'Chat', 'le chat dort');
  await page.goto('/#/practice');
  await expect(page.getByLabel('Zone de frappe')).toBeFocused();
  await page.keyboard.type('le c');

  await page.keyboard.press('Escape');
  await expect(page.getByText(/En pause/)).toBeVisible();
  await expect(page.getByLabel('Zone de frappe')).toBeDisabled();
  await page.keyboard.press('Escape');
  await expect(page.getByText(/En pause/)).toBeHidden();

  await page.keyboard.press('Control+r');
  // La page n'est pas rechargée : le texte collé est toujours là et la session repart de zéro.
  await expect(page.getByLabel('Zone de frappe')).toBeFocused();
  await expect(page.locator('[data-cell="1"]')).toHaveCount(0);
  await page.keyboard.type('le');
  await expect(page.locator('[data-cell="1"]')).toHaveCount(2);
});

test('sans texte, la page renvoie vers la bibliothèque', async ({ page }) => {
  await page.goto('/#/practice');
  await expect(page.getByText('Aucun texte à recopier')).toBeVisible();
  await page.getByRole('link', { name: 'Ouvrir la bibliothèque' }).click();
  await expect(page.getByRole('heading', { name: 'Bibliothèque' })).toBeVisible();
});
