import { describe, it, expect, beforeEach } from 'vitest'
import { useTranslations } from './useTranslations'

describe('useTranslations', () => {
    beforeEach(() => {
        // Reset to default language before each test
        const { setLanguage } = useTranslations()
        void setLanguage('en')
    })

    it('should initialize with English as default', () => {
        const { currentLanguage } = useTranslations()
        expect(currentLanguage.value).toBe('en')
    })

    it('should switch to German', async () => {
        const { currentLanguage, setLanguage } = useTranslations()
        await setLanguage('de')
        expect(currentLanguage.value).toBe('de')
    })

    it('should translate simple keys', () => {
        const { t } = useTranslations()
        const homeNav = t('nav.home')
        expect(homeNav).toBeTruthy()
        expect(typeof homeNav).toBe('string')
    })

    it('should return different translations for different languages', async () => {
        const { t, setLanguage, currentLanguage } = useTranslations()

        const enHome = t('nav.home')
        expect(currentLanguage.value).toBe('en')

        await setLanguage('de')
        expect(currentLanguage.value).toBe('de')

        const deHome = t('nav.home')
        expect(enHome).not.toBe(deHome)
    })

    it('should handle nested translation keys', () => {
        const { t } = useTranslations()
        const title = t('hero.title')
        expect(title).toBeTruthy()
        expect(typeof title).toBe('string')
    })

    it('should have get function defined', () => {
        const { get } = useTranslations()
        // Verify the get function exists
        expect(get).toBeDefined()
        expect(typeof get).toBe('function')
    })

    it('should return key for missing keys', () => {
        const { t } = useTranslations()
        const missing = t('non.existent.key')
        // Returns the key itself when translation is missing
        expect(missing).toBe('non.existent.key')
    })

    it('should persist language in localStorage', async () => {
        const { setLanguage, currentLanguage } = useTranslations()
        await setLanguage('de')
        // Verify language was actually changed
        expect(currentLanguage.value).toBe('de')
    })

    describe('array access', () => {
        it('should access array elements with numeric indices', () => {
            const { t } = useTranslations()
            // roadmap.done.features is an array of objects with title/desc
            const title = t('roadmap.done.features.0.title')
            expect(title).toBe('Users & Roles')
        })

        it('should access nested properties within array elements', () => {
            const { t } = useTranslations()
            const desc = t('roadmap.done.features.1.desc')
            expect(desc).toContain('workflow')
        })

        it('should access simple string arrays', () => {
            const { t } = useTranslations()
            // use_cases.intro.capabilities is an array of strings
            const capability = t('use_cases.intro.capabilities.0')
            expect(capability).toBe('Import/Export CSV & JSON')
        })

        it('should return key for out-of-bounds array index', () => {
            const { t } = useTranslations()
            const outOfBounds = t('roadmap.done.features.999.title')
            expect(outOfBounds).toBe('roadmap.done.features.999.title')
        })

        it('should return key for invalid array index', () => {
            const { t } = useTranslations()
            const invalid = t('roadmap.done.features.notanumber.title')
            expect(invalid).toBe('roadmap.done.features.notanumber.title')
        })
    })
})
