import vue from 'eslint-plugin-vue'
import prettier from 'eslint-plugin-prettier'
import recommended from '@eslint/js'
import prettierConfig from 'eslint-config-prettier'
import vueParser from 'vue-eslint-parser'
import typescriptParser from '@typescript-eslint/parser'
import typescriptPlugin from '@typescript-eslint/eslint-plugin'

export default [
    {
        files: ['*.js', '*.vue', '*.ts', 'src/**/*.{js,vue,ts}'],
        languageOptions: {
            parser: vueParser,
            parserOptions: {
                parser: typescriptParser,
                ecmaVersion: 'latest',
                sourceType: 'module',
                extraFileExtensions: ['.vue'],
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
            // ESLint recommended rules
            ...recommended.rules,

            // TypeScript ESLint recommended rules
            ...typescriptPlugin.configs.recommended.rules,

            // Prettier plugin recommended rules
            ...prettierConfig.rules,

            // Vue.js plugin recommended rules
            ...vue.configs['vue3-recommended'].rules,

            // Custom rules
            'vue/html-indent': ['error', 4],
            indent: 'off', // Prettier handles indentation
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
            curly: ['error', 'all'],
            eqeqeq: ['error', 'smart'],
            quotes: ['error', 'single'],
            semi: ['error', 'never'],

            // TypeScript-specific rules for best practices
            '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
            '@typescript-eslint/explicit-function-return-type': 'off',
            '@typescript-eslint/explicit-module-boundary-types': 'off',
            '@typescript-eslint/no-explicit-any': 'warn',
            '@typescript-eslint/no-inferrable-types': 'error',

            // Disable conflicting ESLint rules in favor of TypeScript equivalents
            'no-unused-vars': 'off',
            'no-undef': 'off', // TypeScript compiler handles this
            'prefer-const': 'error', // Use the base ESLint rule instead
            'prettier/prettier': [
                'error',
                {
                    tabWidth: 4,
                    singleAttributePerLine: true,
                    vueIndentScriptAndStyle: true,
                    bracketSameLine: false,
                },
            ],
        },
    },
]
