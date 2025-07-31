export default {
    preset: 'ts-jest',
    testEnvironment: 'jsdom',
    moduleNameMapper: {
        '^@/(.*)$': '<rootDir>/src/$1',
    },
    transform: {
        '^.+\\.ts$': [
            'ts-jest',
            {
                tsconfig: 'tsconfig.test.json',
            },
        ],
    },
    moduleFileExtensions: ['ts', 'js', 'json'],
    testMatch: ['**/__tests__/**/*.ts', '**/?(*.)+(spec|test).ts', '**/src/**/*.test.ts'],
    collectCoverageFrom: [
        'src/**/*.{ts,vue}',
        '!src/**/*.d.ts',
        '!src/main.ts',
        '!src/env-check.ts',
        '!src/**/*.test.ts',
    ],
    setupFilesAfterEnv: ['<rootDir>/src/test-setup.ts'],
    testPathIgnorePatterns: ['/node_modules/', '/dist/'],
    moduleDirectories: ['node_modules', 'src'],
    roots: ['<rootDir>/src'],
    testEnvironmentOptions: {
        url: 'http://localhost',
    },
}
