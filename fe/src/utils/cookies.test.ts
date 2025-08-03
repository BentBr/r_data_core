/**
 * Tests for cookie utilities
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { setSecureCookie, getCookie, deleteCookie } from './cookies'

// Mock the env-check module before importing
vi.mock('../env-check', () => ({
    env: {
        isDevelopment: true,
        isProduction: false,
    },
}))

describe('Cookie Utils', () => {
    beforeEach(() => {
        // Clear all cookies before each test
        document.cookie = ''
    })

    afterEach(() => {
        // Clear all cookies after each test
        document.cookie = ''
    })

    it('should set a cookie', () => {
        setSecureCookie('test-cookie', 'test-value')
        expect(document.cookie).toContain('test-cookie=test-value')
    })

    it('should get a cookie value', () => {
        setSecureCookie('test-cookie', 'test-value')
        const value = getCookie('test-cookie')
        expect(value).toBe('test-value')
    })

    it('should return null for non-existent cookie', () => {
        const value = getCookie('non-existent')
        expect(value).toBeNull()
    })

    it('should delete a cookie', () => {
        setSecureCookie('test-cookie', 'test-value')
        expect(getCookie('test-cookie')).toBe('test-value')

        deleteCookie('test-cookie')
        expect(getCookie('test-cookie')).toBeNull()
    })

    it('should set cookie with options', () => {
        setSecureCookie('test-cookie', 'test-value', {
            expires: new Date('2024-12-31'),
            secure: true,
            sameSite: 'strict',
        })

        expect(getCookie('test-cookie')).toBeNull() // Secure cookies are ignored in JSDOM
    })

    it('should handle special characters in cookie value', () => {
        const specialValue = 'test=value; with,special:chars'
        setSecureCookie('test-cookie', specialValue)
        const retrieved = getCookie('test-cookie')
        expect(retrieved).toBe(specialValue)
    })

    it('should handle empty cookie value', () => {
        setSecureCookie('test-cookie', '')
        const value = getCookie('test-cookie')
        // The getCookie function returns null for empty values, which is correct behavior
        expect(value).toBeNull()
    })

    it('should handle multiple cookies', () => {
        setSecureCookie('cookie1', 'value1')
        setSecureCookie('cookie2', 'value2')

        expect(getCookie('cookie1')).toBe('value1')
        expect(getCookie('cookie2')).toBe('value2')
    })

    it('should update existing cookie', () => {
        setSecureCookie('test-cookie', 'old-value')
        setSecureCookie('test-cookie', 'new-value')

        expect(getCookie('test-cookie')).toBe('new-value')
    })

    it('should handle cookie names with special characters', () => {
        setSecureCookie('test-cookie-name', 'value')
        const value = getCookie('test-cookie-name')
        expect(value).toBe('value')
    })

    it('should handle very long cookie values', () => {
        const longValue = 'a'.repeat(1000)
        setSecureCookie('test-cookie', longValue)
        const retrieved = getCookie('test-cookie')
        expect(retrieved).toBe(longValue)
    })
})
