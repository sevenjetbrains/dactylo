/** Point d'entrée de l'interface : traductions puis rendu de l'application. */

import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import { App } from './app/App';
import { modules } from './app/registry';
import { initI18n } from './i18n';
import './index.css';

// Les traductions doivent être prêtes avant le premier rendu.
initI18n(modules);

// Conteneur défini dans index.html ; son absence est une erreur de construction.
const root = document.getElementById('root');
if (!root) throw new Error('Élément #root absent de index.html');

// StrictMode signale en développement les effets mal nettoyés.
createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
