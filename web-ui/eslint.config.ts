import { defineConfig, globalIgnores } from 'eslint/config'
import pluginVitest from '@vitest/eslint-plugin'
import prettierConfig from 'eslint-config-prettier/flat'
import tseslint from 'typescript-eslint'

export default defineConfig(
  globalIgnores(['../dist/**', 'coverage/**', 'private/**']),
  ...tseslint.configs.recommended,
  {
    files: ['src/**/*.{ts,tsx}'],
    languageOptions: {
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
  },
  {
    ...pluginVitest.configs.recommended,
    files: ['src/test/**/*.{ts,tsx}'],
  },
  prettierConfig,
)
