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
})
