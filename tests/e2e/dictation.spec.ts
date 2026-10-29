/** Parcours de la dictée de bout en bout, sur le faux backend. */

import { expect, test } from '@playwright/test';

import { installFakeBackend } from './fake-backend';

// Chaque test tourne contre le faux backend IPC.
test.beforeEach(async ({ page }) => {
  await installFakeBackend(page);
});

test('dicter un paragraphe, le taper à l’aveugle et lire la correction', async ({ page }) => {
  // Texte de 6 mots : deux segments, de 4 puis 2 mots.
  await page.goto('/#/library?view=paste');
  await page.getByLabel('Titre').fill('Le chat');
  await page.getByLabel('Texte (collez-le ici)').fill('Le chat noir dort sur le tapis');
  await page.getByRole('button', { name: 'Enregistrer' }).click();
  await expect(page.getByRole('status')).toContainText('« Le chat » a été ajouté.');

  await page.goto('/#/dictation');
  await expect(page.getByRole('heading', { name: 'Le chat' })).toBeVisible();
  // Le texte n'apparaît nulle part avant la correction.
  await expect(page.getByText('noir')).toHaveCount(0);
  await page.getByRole('button', { name: 'Commencer la dictée' }).click();
  await expect(page.getByText(/Segment 1 sur 2 · à vous de taper/)).toBeVisible();

  // Quatre mots tapés : le segment suivant est lu (avancement adaptatif).
  const copy = page.getByLabel('Votre copie');
  await copy.pressSequentially('Le chat noir dort ');
  await expect(page.getByText(/Segment 2 sur 2/)).toBeVisible();
  await copy.pressSequentially('sur le tapi');
  await page.getByRole('button', { name: 'Terminer et corriger' }).click();

  // Correction : une faute sur 7 mots, mot tapé et mot attendu visibles.
  await expect(page.getByRole('heading', { name: /1 faute sur 7 mots/ })).toBeVisible();
  await expect(page.locator('del', { hasText: 'tapi' })).toBeVisible();
  await expect(page.locator('ins', { hasText: 'tapis' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Mots ajoutés à vos révisions' })).toBeVisible();
});
