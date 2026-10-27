import js from '@eslint/js';
import reactHooks from 'eslint-plugin-react-hooks';
import reactRefresh from 'eslint-plugin-react-refresh';
import globals from 'globals';
import tseslint from 'typescript-eslint';

const crossModuleMessage =
  'Un module ne doit pas importer un autre module : passer par les événements (ADR 0006).';

export default tseslint.config(
  {
    ignores: [
      'dist',
      'coverage',
      'target',
      'src-tauri',
      'playwright-report',
      'test-results',
      'src/shared/api/bindings.ts',
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.strict,
  {
    files: ['**/*.{ts,tsx}'],
    languageOptions: { globals: globals.browser },
    plugins: { 'react-hooks': reactHooks, 'react-refresh': reactRefresh },
    rules: {
      ...reactHooks.configs.recommended.rules,
      'react-refresh/only-export-components': ['warn', { allowConstantExport: true }],
    },
  },
  {
    // Dans un module : imports locaux (./) ou via l'alias @/shared, jamais un autre module.
    files: ['src/modules/**/*.{ts,tsx}'],
    rules: {
      'no-restricted-imports': [
        'error',
        { patterns: [{ group: ['@/modules/**', '../**'], message: crossModuleMessage }] },
      ],
    },
  },
  {
    // Seul le registre connaît les modules (ADR 0005).
    files: ['src/app/**/*.{ts,tsx}', 'src/shared/**/*.{ts,tsx}'],
    ignores: ['src/app/registry.ts'],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          patterns: [
            { group: ['@/modules/**', '**/modules/**'], message: 'Passer par le registre.' },
          ],
        },
      ],
    },
  },
  {
    files: ['*.config.{js,ts}', 'tests/**'],
    languageOptions: { globals: globals.node },
  },
);
