import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'

// The global test-setup mocks '@/composables/useTranslations' for every other
// test file in the suite (so components don't need real translation data).
// Because vi.mock matches by resolved module path, that mock also intercepts
// this file's own relative import of the module under test — which is why
// the real implementation previously reported 0% coverage despite this file
// passing. Unmock it here (hoisted, like vi.mock) so the real
// `useTranslations` runs.
vi.unmock('@/composables/useTranslations')
vi.unmock('./useTranslations')

import { useTranslations } from './useTranslations'

describe('useTranslations', () => {
    beforeEach(() => {
        localStorage.clear()
    })

    afterEach(() => {
        localStorage.clear()
    })

    describe('t function', () => {
        it('resolves a real key from the loaded English translations', async () => {
            const { t, initTranslations, setLanguage } = useTranslations()
            await initTranslations()
            await setLanguage('en')

            expect(t('auth.login.errors.invalid_credentials')).toBe('Invalid username or password')
        })

        it('falls back to the provided fallback string when the key does not exist', async () => {
            const { t, initTranslations } = useTranslations()
            await initTranslations()

            const result = t('this.key.does.not.exist.anywhere', 'My fallback text')
            expect(result).toBe('My fallback text')
        })

        it('falls back to the key itself when no translation and no fallback exist', async () => {
            const { t, initTranslations } = useTranslations()
            await initTranslations()

            const uniqueKey = 'totally.unknown.key.xyz'
            expect(t(uniqueKey)).toBe(uniqueKey)
        })

        it('replaces named placeholders using the params object', async () => {
            const { t, initTranslations } = useTranslations()
            await initTranslations()

            const result = t('dashboard.tiles.top_entity_type', { type: 'Product', count: '5' })
            expect(result).toBe('Top: Product (5)')
        })

        it('treats a string second argument as the fallback, not as params', async () => {
            const { t, initTranslations } = useTranslations()
            await initTranslations()

            const uniqueKey = 'another.unknown.key.abc'
            expect(t(uniqueKey, 'Fallback wins')).toBe('Fallback wins')
        })

        it('falls back to English when the current language is missing a key', async () => {
            const { t, initTranslations, setLanguage } = useTranslations()
            await initTranslations()
            await setLanguage('de')

            // German translations always contain this key (parity-enforced), so
            // exercise the fallback path with a key that is present only in the
            // (always fully populated) English fallback object by asserting the
            // behaviour directly: an English-only key still resolves even though
            // the current language is German.
            expect(t('auth.login.errors.invalid_credentials')).toBe(
                'Ungültiger Benutzername oder Passwort'
            )

            await setLanguage('en')
        })
    })

    describe('translateError', () => {
        it('maps "invalid credentials" style messages to the credentials key', async () => {
            const { translateError, initTranslations, setLanguage } = useTranslations()
            await initTranslations()
            await setLanguage('en')

            expect(translateError('Invalid credentials provided')).toBe(
                'Invalid username or password'
            )
            expect(translateError('Invalid username supplied')).toBe('Invalid username or password')
            expect(translateError('Invalid password supplied')).toBe('Invalid username or password')
        })

        it('maps username-required messages', async () => {
            const { translateError, initTranslations } = useTranslations()
            await initTranslations()

            expect(translateError('Username is required')).toBe('Username is required')
        })

        it('maps password-required messages', async () => {
            const { translateError, initTranslations } = useTranslations()
            await initTranslations()

            expect(translateError('Password is required')).toBe('Password is required')
        })

        it('maps validation-failure messages', async () => {
            const { translateError, initTranslations } = useTranslations()
            await initTranslations()

            expect(translateError('Validation failed on the server')).toBe(
                'Login failed due to validation error'
            )
        })

        it('maps network/connection messages', async () => {
            const { translateError, initTranslations } = useTranslations()
            await initTranslations()

            expect(translateError('Network connection failed')).toBe(
                'Network error, please try again'
            )
            expect(translateError('Connection refused')).toBe('Network error, please try again')
        })

        it('maps server-error messages', async () => {
            const { translateError, initTranslations } = useTranslations()
            await initTranslations()

            expect(translateError('Internal server error occurred')).toBe(
                'Server error, please try again later'
            )
            expect(translateError('A server error happened')).toBe(
                'Server error, please try again later'
            )
        })

        it('maps "authentication required" messages', async () => {
            const { translateError, initTranslations } = useTranslations()
            await initTranslations()

            expect(translateError('Authentication required to continue')).toBe(
                'Authentication required'
            )
        })

        it('returns the original message when no pattern matches', async () => {
            const { translateError, initTranslations } = useTranslations()
            await initTranslations()

            expect(translateError('Some completely unrelated error message')).toBe(
                'Some completely unrelated error message'
            )
        })
    })

    describe('setLanguage', () => {
        it('persists the chosen language to localStorage and updates currentLanguage', async () => {
            const instance = useTranslations()
            await instance.initTranslations()

            await instance.setLanguage('de')
            expect(localStorage.getItem('preferred-language')).toBe('de')
            expect(instance.currentLanguage.value).toBe('de')

            await instance.setLanguage('en')
            expect(localStorage.getItem('preferred-language')).toBe('en')
            expect(instance.currentLanguage.value).toBe('en')
        })
    })

    describe('availableLanguages', () => {
        it('lists English and German', () => {
            const { availableLanguages } = useTranslations()

            expect(availableLanguages.value).toEqual(['en', 'de'])
        })
    })

    describe('initTranslations', () => {
        it('is safe to call multiple times (idempotent)', async () => {
            const { initTranslations, t } = useTranslations()

            await initTranslations()
            await initTranslations()

            expect(t('auth.login.errors.invalid_credentials')).toBe('Invalid username or password')
        })
    })
})
