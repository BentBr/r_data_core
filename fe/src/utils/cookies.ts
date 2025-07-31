/**
 * Secure cookie utilities for handling refresh tokens
 */

import { env } from '../env-check'

interface CookieOptions {
    expires?: Date
    path?: string
    domain?: string
    secure?: boolean
    sameSite?: 'strict' | 'lax' | 'none'
    httpOnly?: boolean
}

const DEFAULT_OPTIONS: CookieOptions = {
    path: '/',
    secure: !env.isDevelopment, // Only secure in production
    sameSite: 'strict', // Protect against CSRF
    httpOnly: false, // Allow JavaScript access for refresh tokens
}

/**
 * Set a secure cookie
 */
export function setSecureCookie(name: string, value: string, options: CookieOptions = {}): void {
    const opts = { ...DEFAULT_OPTIONS, ...options }

    let cookieString = `${encodeURIComponent(name)}=${encodeURIComponent(value)}`

    if (opts.expires) {
        cookieString += `; expires=${opts.expires.toUTCString()}`
    }

    if (opts.path) {
        cookieString += `; path=${opts.path}`
    }

    if (opts.domain) {
        cookieString += `; domain=${opts.domain}`
    }

    if (opts.secure) {
        cookieString += '; secure'
    }

    if (opts.sameSite) {
        cookieString += `; samesite=${opts.sameSite}`
    }

    document.cookie = cookieString
}

/**
 * Get a cookie value
 */
export function getCookie(name: string): string | null {
    const nameEQ = `${encodeURIComponent(name)}=`
    const cookies = document.cookie.split(';')

    for (let i = 0; i < cookies.length; i++) {
        let cookie = cookies[i]
        while (cookie.charAt(0) === ' ') {
            cookie = cookie.substring(1, cookie.length)
        }
        if (cookie.indexOf(nameEQ) === 0) {
            const value = decodeURIComponent(cookie.substring(nameEQ.length, cookie.length))
            return value || null // Return null for empty values
        }
    }
    return null
}

/**
 * Delete a cookie
 */
export function deleteCookie(name: string, options: CookieOptions = {}): void {
    const opts = { ...DEFAULT_OPTIONS, ...options }
    opts.expires = new Date(0) // Set to past date to expire immediately
    setSecureCookie(name, '', opts)
}

/**
 * Check if cookies are supported
 */
export function areCookiesSupported(): boolean {
    try {
        document.cookie = 'test=1'
        const supported = document.cookie.indexOf('test=') !== -1
        document.cookie = 'test=1; expires=Thu, 01 Jan 1970 00:00:00 GMT'
        return supported
    } catch {
        return false
    }
}

/**
 * Set refresh token as secure cookie
 */
export function setRefreshToken(token: string, expiresAt: Date): void {
    setSecureCookie('refresh_token', token, {
        expires: expiresAt,
        secure: !env.isDevelopment, // Only secure in production
        sameSite: 'strict',
    })
}

/**
 * Get refresh token from secure cookie
 */
export function getRefreshToken(): string | null {
    return getCookie('refresh_token')
}

/**
 * Delete refresh token cookie
 */
export function deleteRefreshToken(): void {
    deleteCookie('refresh_token')
}
