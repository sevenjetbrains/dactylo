/** Fin de parcours de la phase 2 : import des pages 3 à 5 d'un PDF, nettoyage, exercice. */

import { fileURLToPath } from 'node:url';

import { expect, test } from '@playwright/test';

import { installFakeBackend } from './fake-backend';

/** PDF de six pages généré par `fixtures/make-pdf.py`. */
const PDF = fileURLToPath(new URL('./fixtures/livre.pdf', import.meta.url));

// Chaque test tourne contre le faux backend IPC ; pdf.js, lui, est le vrai.
test.beforeEach(async ({ page }) => {
  await installFakeBackend(page);
});

test('importer les pages 3 à 5 d’un PDF, les nettoyer et s’entraîner exercice par exercice', async ({
  page,
}) => {
  await page.goto('/#/library?view=pdf');
  // Ouverture du PDF d'essai.
  await page.getByLabel('Fichier PDF').setInputFiles(PDF);
  await expect(page.getByText('(6 pages)')).toBeVisible();
  await expect(page.getByLabel('Aperçu de la page 1')).toBeVisible();

  // Pages 3 à 5 : pdf.js extrait les lignes, sans en-têtes ni numéros de page.
  await page.getByLabel('De la page', { exact: true }).fill('3');
  await page.getByLabel('à la page', { exact: true }).fill('5');
  await page.getByRole('button', { name: 'Afficher les lignes' }).click();
  await expect(page.getByLabel('Ligne 1, page 3')).toContainText(
    'Page trois : Phileas Fogg était un homme fort exact',
  );
  await expect(page.getByLabel('Ligne 6, page 5')).toBeVisible();

  // Texte nettoyé : la césure « pres- / sait » est recollée.
  const content = page.getByLabel('Texte nettoyé (modifiable)');
  await expect(content).toHaveValue(/qui ne se pressait jamais/);

  // Un exercice par paragraphe (un par page ici), puis enregistrement.
  await page.getByLabel('Titre').fill('Le livre');
  await page.getByLabel('Découpage en exercices').selectOption('paragraphs');
  await page.getByRole('button', { name: 'Enregistrer le texte' }).click();
  await expect(page.getByRole('status')).toContainText('« Le livre » a été ajouté.');
  await expect(page.getByText('PDF, p. 3-5')).toBeVisible();
  await expect(page.getByText('Exercice 1 sur 3')).toBeVisible();

  // Premier exercice : seule la page 3 est à taper.
  await page.getByRole('link', { name: "S'entraîner sur « Le livre »" }).click();
  await expect(page.getByText('Copie · Exercice 1 sur 3')).toBeVisible();
  await expect(page.getByLabel('Zone de frappe')).toBeFocused();
  await page.keyboard.type(
    "Page trois : Phileas Fogg était un homme fort exact, qui ne se pressait jamais et arrivait toujours à l'heure, page trois.",
  );
  await expect(page.getByRole('heading', { name: 'Session terminée' })).toBeVisible();

  // L'écran de fin enchaîne sur l'exercice suivant.
  await page.getByRole('button', { name: 'Exercice suivant (2 sur 3)' }).click();
  await expect(page.getByText('Copie · Exercice 2 sur 3')).toBeVisible();
});
