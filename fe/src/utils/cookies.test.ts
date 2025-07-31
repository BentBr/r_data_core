/**
 * Tests for cookie utilities
 */

// Mock the env-check module before importing
jest.mock('../env-check', () => ({
    env: {
        isDevelopment: true,
        isProduction: false,
    },
}))

// Now import the cookies module
import {
    setSecureCookie,
    getCookie,
    deleteCookie,
    areCookiesSupported,
    setRefreshToken,
    getRefreshToken,
    deleteRefreshToken,
} from './cookies'

describe('Cookie Utilities', () => {
    beforeEach(() => {
        // Clear cookies before each test
        document.cookie = ''
        // Also clear any existing cookies by setting them to expire
        const cookies = document.cookie.split(';')
        cookies.forEach(cookie => {
            const name = cookie.split('=')[0].trim()
            if (name) {
                document.cookie = `${name}=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/`
            }
        })
    })

    test('setSecureCookie should set a cookie with proper attributes', () => {
        setSecureCookie('test_cookie', 'test_value', {
            expires: new Date('2024-12-31'),
            secure: false, // Force false for testing
            sameSite: 'strict',
        })

        expect(document.cookie).toContain('test_cookie=test_value')
        expect(document.cookie).toContain('samesite=strict')
        expect(document.cookie).toContain('expires=')
        // Should not contain secure in development
        expect(document.cookie).not.toContain('secure')
    })

    test('setSecureCookie should include secure flag when secure is true', () => {
        setSecureCookie('test_cookie', 'test_value', {
            secure: true, // Force true for testing
            sameSite: 'strict',
        })

        expect(document.cookie).toContain('test_cookie=test_value')
        expect(document.cookie).toContain('secure')
        expect(document.cookie).toContain('samesite=strict')
    })

    test('getCookie should retrieve cookie value', () => {
        setSecureCookie('test_cookie', 'test_value')

        const value = getCookie('test_cookie')
        expect(value).toBe('test_value')
    })

    test('getCookie should return null for non-existent cookie', () => {
        const value = getCookie('non_existent')
        expect(value).toBeNull()
    })

    test('deleteCookie should remove cookie', () => {
        setSecureCookie('test_cookie', 'test_value')
        expect(getCookie('test_cookie')).toBe('test_value')

        deleteCookie('test_cookie')
        expect(getCookie('test_cookie')).toBeNull()
    })

    test('areCookiesSupported should return true when cookies work', () => {
        const supported = areCookiesSupported()
        expect(supported).toBe(true)
    })

    test('setRefreshToken should set refresh token with proper attributes', () => {
        const expiresAt = new Date('2024-12-31')
        setRefreshToken('test_refresh_token', expiresAt)

        expect(document.cookie).toContain('refresh_token=test_refresh_token')
        expect(document.cookie).toContain('samesite=strict')
        expect(document.cookie).toContain('expires=')
        // Should not contain secure in development
        expect(document.cookie).not.toContain('secure')
    })

    test('getRefreshToken should retrieve refresh token', () => {
        setRefreshToken('test_refresh_token', new Date('2024-12-31'))

        const value = getRefreshToken()
        expect(value).toBe('test_refresh_token')
    })

    test('deleteRefreshToken should remove refresh token', () => {
        setRefreshToken('test_refresh_token', new Date('2024-12-31'))
        expect(getRefreshToken()).toBe('test_refresh_token')

        deleteRefreshToken()
        expect(getRefreshToken()).toBeNull()
    })

    test('cookie encoding should handle special characters', () => {
        setSecureCookie('test_cookie', 'test=value;with;special;chars')

        expect(document.cookie).toContain('test_cookie=test%3Dvalue%3Bwith%3Bspecial%3Bchars')
    })

    test('cookie decoding should handle special characters', () => {
        setSecureCookie('test_cookie', 'test=value;with;special;chars')

        const value = getCookie('test_cookie')
        expect(value).toBe('test=value;with;special;chars')
    })
})
