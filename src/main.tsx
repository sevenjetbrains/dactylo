import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import { App } from './app/App';
import { modules } from './app/registry';
import { initI18n } from './i18n';
import './index.css';

initI18n(modules);

const root = document.getElementById('root');
if (!root) throw new Error('Élément #root absent de index.html');

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
