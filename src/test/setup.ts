import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterEach } from 'vitest';

import { modules } from '@/app/registry';
import { initI18n } from '@/i18n';

initI18n(modules);

afterEach(() => cleanup());
