import vue from 'eslint-plugin-vue'
import prettier from 'eslint-plugin-prettier'
import recommended from '@eslint/js'
import prettierConfig from 'eslint-config-prettier'
import vueParser from 'vue-eslint-parser'

export default [
    {
        files: ['*.js', '*.vue', 'src/**/*.{js,vue}'],
        languageOptions: {
            parser: vueParser,
            ecmaVersion: 'latest',
            sourceType: 'module',
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
        },
        rules: {
            // ESLint recommended rules
            ...recommended.rules,

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
