import vue from 'eslint-plugin-vue'
import prettier from 'eslint-plugin-prettier'
import recommended from '@eslint/js'
import prettierConfig from 'eslint-config-prettier'
import vueParser from 'vue-eslint-parser'
import typescriptParser from '@typescript-eslint/parser'
import typescriptPlugin from '@typescript-eslint/eslint-plugin'

// Shared rule constants
const sharedTypescriptRules = {
    ...typescriptPlugin.configs.recommended.rules,
    '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
    '@typescript-eslint/explicit-function-return-type': 'off',
    '@typescript-eslint/explicit-module-boundary-types': 'off',
    '@typescript-eslint/no-explicit-any': 'warn',
    '@typescript-eslint/no-inferrable-types': 'error',
    '@typescript-eslint/no-misused-promises': 'error',
    '@typescript-eslint/no-floating-promises': 'error',
    '@typescript-eslint/prefer-nullish-coalescing': 'error',
    '@typescript-eslint/prefer-optional-chain': 'error',
    '@typescript-eslint/no-unnecessary-condition': 'error',
}

const sharedBaseRules = {
    curly: ['error', 'all'],
    eqeqeq: ['error', 'smart'],
    quotes: ['error', 'single'],
    semi: ['error', 'never'],
    'no-unused-vars': 'off',
    'no-undef': 'off',
    'prefer-const': 'error',
}

const prettierRules = {
    ...prettierConfig.rules,
    'prettier/prettier': [
        'error',
        {
            tabWidth: 4,
            singleAttributePerLine: true,
            vueIndentScriptAndStyle: true,
            bracketSameLine: false,
        },
    ],
}

export default [
    // Source files (excluding test files)
    {
        files: ['src/**/*.{js,vue,ts}'],
        ignores: ['**/*.test.{js,ts}', '**/*.spec.{js,ts}'],
        languageOptions: {
            parser: vueParser,
            parserOptions: {
                parser: typescriptParser,
                ecmaVersion: 'latest',
                sourceType: 'module',
                extraFileExtensions: ['.vue'],
                project: ['./tsconfig.json'],
                tsconfigRootDir: import.meta.dirname,
            },
            globals: {
                browser: true,
                es2021: true,
                node: true,
            },
        },
        plugins: {
            vue,
            prettier,
            recommended,
            '@typescript-eslint': typescriptPlugin,
        },
        rules: {
            ...recommended.rules,
            ...sharedTypescriptRules,
            ...sharedBaseRules,
            ...prettierRules,
            ...vue.configs['vue3-recommended'].rules,
            'vue/html-indent': 'off',
            indent: 'off',
            'no-alert': 0,
            'vue/html-self-closing': [
                'error',
                {
                    html: {
                        void: 'any',
                        normal: 'any',
                        component: 'any',
                    },
                    svg: 'any',
                    math: 'any',
                },
            ],
            'vue/no-v-html': 0,
            'vue/require-prop-types': 'off',
            'vue/require-default-prop': 'off',
            'vue/multi-word-component-names': 'off',
        },
    },
    // Unit test spec files (src/**/*.test.ts) — relaxed any, type-aware
    {
        files: ['src/**/*.test.{js,ts}'],
        languageOptions: {
            parser: typescriptParser,
            parserOptions: {
                ecmaVersion: 'latest',
                sourceType: 'module',
                project: ['./tsconfig.json'],
                tsconfigRootDir: import.meta.dirname,
            },
        },
        plugins: {
            '@typescript-eslint': typescriptPlugin,
            prettier,
        },
        rules: {
            ...sharedTypescriptRules,
            ...sharedBaseRules,
            ...prettierRules,
            '@typescript-eslint/no-explicit-any': 'off',
        },
    },
    // E2E non-test files (helpers, page objects, fixtures, setup/teardown)
    {
        files: ['e2e/**/*.ts'],
        ignores: ['e2e/**/*.test.ts'],
        languageOptions: {
            parser: typescriptParser,
            parserOptions: {
                ecmaVersion: 'latest',
                sourceType: 'module',
                project: ['./e2e/tsconfig.json'],
                tsconfigRootDir: import.meta.dirname,
            },
        },
        plugins: {
            '@typescript-eslint': typescriptPlugin,
            prettier,
        },
        rules: {
            ...sharedTypescriptRules,
            ...sharedBaseRules,
            ...prettierRules,
        },
    },
    // E2E test files — relaxed any
    {
        files: ['e2e/**/*.test.ts'],
        languageOptions: {
            parser: typescriptParser,
            parserOptions: {
                ecmaVersion: 'latest',
                sourceType: 'module',
                project: ['./e2e/tsconfig.json'],
                tsconfigRootDir: import.meta.dirname,
            },
        },
        plugins: {
            '@typescript-eslint': typescriptPlugin,
            prettier,
        },
        rules: {
            ...sharedTypescriptRules,
            ...sharedBaseRules,
            ...prettierRules,
            '@typescript-eslint/no-explicit-any': 'off',
        },
    },
    // Config files (without type-checking)
    {
        files: ['*.config.{js,ts}', 'eslint.config.js'],
        languageOptions: {
            parser: typescriptParser,
            ecmaVersion: 'latest',
            sourceType: 'module',
        },
        plugins: {
            '@typescript-eslint': typescriptPlugin,
            prettier,
        },
        rules: {
            ...prettierRules,
            '@typescript-eslint/no-explicit-any': 'warn',
        },
    },
]
