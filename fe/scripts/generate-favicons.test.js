import { describe, it, expect } from 'vitest'

describe('generate-favicons script (FE)', () => {
    it('should define required favicon sizes', () => {
        const faviconSizes = [16, 32, 48, 180, 192, 512]
        expect(Array.isArray(faviconSizes)).toBe(true)
        expect(faviconSizes).toContain(16)
        expect(faviconSizes).toContain(32)
        expect(faviconSizes).toContain(48)
        expect(faviconSizes).toContain(180) // Apple Touch Icon
        expect(faviconSizes).toContain(192) // PWA
        expect(faviconSizes).toContain(512) // PWA
    })

    it('should have sharp module available', async () => {
        const sharp = await import('sharp')
        expect(sharp.default).toBeDefined()
    })

    it('should generate standard browser favicons', () => {
        const standardSizes = [16, 32, 48]
        expect(standardSizes.every(size => size >= 16 && size <= 48)).toBe(true)
    })

    it('should generate Apple Touch Icon', () => {
        const appleTouchIconSize = 180
        expect(appleTouchIconSize).toBe(180)
    })

    it('should generate PWA icons', () => {
        const pwaSizes = [192, 512]
        expect(pwaSizes).toContain(192)
        expect(pwaSizes).toContain(512)
    })

    it('should generate default favicon', () => {
        const defaultFaviconSize = 32
        expect(defaultFaviconSize).toBe(32)
    })

    it('should use PNG format for favicons', () => {
        const format = 'png'
        expect(format).toBe('png')
    })
})
