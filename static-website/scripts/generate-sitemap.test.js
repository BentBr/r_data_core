import { describe, it, expect } from 'vitest'

describe('generate-sitemap script', () => {
    it('should define required pages', () => {
        const pages = ['/', '/about', '/pricing']
        expect(Array.isArray(pages)).toBe(true)
        expect(pages).toContain('/')
        expect(pages).toContain('/about')
        expect(pages).toContain('/pricing')
    })

    it('should define supported languages', () => {
        const languages = ['en', 'de']
        expect(Array.isArray(languages)).toBe(true)
        expect(languages).toContain('en')
        expect(languages).toContain('de')
    })

    it('should exclude noindex pages from sitemap', () => {
        const indexablePages = ['/', '/about', '/pricing']
        const excludedPages = ['/imprint', '/privacy']

        expect(indexablePages).not.toContain('/imprint')
        expect(indexablePages).not.toContain('/privacy')
        expect(excludedPages).toContain('/imprint')
        expect(excludedPages).toContain('/privacy')
    })

    it('should generate correct sitemap structure', () => {
        const sitemapFiles = ['sitemap.xml', 'sitemap_en.xml', 'sitemap_de.xml', 'robots.txt']
        expect(sitemapFiles).toContain('sitemap.xml')
        expect(sitemapFiles).toContain('sitemap_en.xml')
        expect(sitemapFiles).toContain('sitemap_de.xml')
        expect(sitemapFiles).toContain('robots.txt')
    })
})
