import { describe, it, expect, beforeEach, vi } from 'vitest'
import { env, getBaseUrl } from './env-check'

describe('env-check', () => {
    beforeEach(() => {
        // Reset environment
        vi.stubEnv('VITE_BASE_URL', '')
        vi.stubEnv('VITE_SITE_NAME', '')
        vi.stubEnv('VITE_R_DATA_CORE_DEMO_URL', '')
    })

    describe('getBaseUrl', () => {
        it('should return VITE_BASE_URL if set', () => {
            vi.stubEnv('VITE_BASE_URL', 'https://example.com')
            const url = getBaseUrl()
            expect(url).toBe('https://example.com')
        })

        it('should return window.location.origin if VITE_BASE_URL not set', () => {
            vi.stubEnv('VITE_BASE_URL', '')
            const url = getBaseUrl()
            expect(url).toBeTruthy()
            expect(typeof url).toBe('string')
        })

        it('should trim whitespace from URL', () => {
            vi.stubEnv('VITE_BASE_URL', '  https://example.com  ')
            const url = getBaseUrl()
            expect(url).toBe('https://example.com')
        })
    })

    describe('env object', () => {
        it('should have baseUrl property', () => {
            expect(env).toHaveProperty('baseUrl')
            expect(typeof env.baseUrl).toBe('string')
        })

        it('should have siteName with default fallback', () => {
            expect(env).toHaveProperty('siteName')
            expect(env.siteName).toBeTruthy()
        })

        it('should have demoUrl property', () => {
            expect(env).toHaveProperty('demoUrl')
            expect(typeof env.demoUrl).toBe('string')
        })

        it('should have apiDocsUrl property', () => {
            expect(env).toHaveProperty('apiDocsUrl')
            expect(typeof env.apiDocsUrl).toBe('string')
        })

        it('should have adminApiDocsUrl property', () => {
            expect(env).toHaveProperty('adminApiDocsUrl')
            expect(typeof env.adminApiDocsUrl).toBe('string')
        })

        it('should have isProduction boolean', () => {
            expect(env).toHaveProperty('isProduction')
            expect(typeof env.isProduction).toBe('boolean')
        })

        it('should have isDevelopment boolean', () => {
            expect(env).toHaveProperty('isDevelopment')
            expect(typeof env.isDevelopment).toBe('boolean')
        })
    })
})
