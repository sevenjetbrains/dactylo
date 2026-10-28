/** Préparation commune à tous les tests Vitest. */

import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterEach } from 'vitest';

import { modules } from '@/app/registry';
import { initI18n } from '@/i18n';

// Traductions de tous les modules chargées, comme dans l'application.
initI18n(modules);

// Démonte les composants rendus entre deux tests.
afterEach(() => cleanup());
