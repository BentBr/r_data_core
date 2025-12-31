import { describe, it, expect, beforeAll } from 'vitest'
import { execSync } from 'child_process'
import { existsSync, readFileSync } from 'fs'
import { join } from 'path'

const distDir = join(process.cwd(), 'dist')

describe('SSG Build', () => {
    beforeAll(() => {
        // Build the SSG output if dist doesn't exist or is empty
        if (!existsSync(join(distDir, 'en.html'))) {
            console.log('Building SSG output...')
            execSync('npm run build', { stdio: 'inherit' })
        }
    }, 300000) // 5 minute timeout for build

    describe('Generated Files', () => {
        it('should generate HTML files for all routes', () => {
            const expectedFiles = [
                'index.html',
                'en.html',
                'de.html',
                'en/about.html',
                'en/pricing.html',
                'en/roadmap.html',
                'en/use-cases.html',
                'en/imprint.html',
                'en/privacy.html',
                'de/about.html',
                'de/pricing.html',
                'de/roadmap.html',
                'de/use-cases.html',
                'de/imprint.html',
                'de/privacy.html',
            ]

            for (const file of expectedFiles) {
                const filePath = join(distDir, file)
                expect(existsSync(filePath), `Expected ${file} to exist`).toBe(true)
            }
        })

        it('should generate sitemaps', () => {
            expect(existsSync(join(distDir, 'sitemap.xml'))).toBe(true)
            expect(existsSync(join(distDir, 'sitemap_en.xml'))).toBe(true)
            expect(existsSync(join(distDir, 'sitemap_de.xml'))).toBe(true)
        })

        it('should generate robots.txt', () => {
            expect(existsSync(join(distDir, 'robots.txt'))).toBe(true)
        })
    })

    describe('SEO Meta Tags - English Home Page', () => {
        let html

        beforeAll(() => {
            html = readFileSync(join(distDir, 'en.html'), 'utf-8')
        })

        it('should have a proper title tag with translated content', () => {
            // Title should NOT be "seo.home.title" (translation key)
            expect(html).not.toContain('<title>seo.home.title')
            // Title should contain actual translated content
            expect(html).toMatch(/<title>[^<]+RDataCore<\/title>/)
        })

        it('should have meta description', () => {
            expect(html).toMatch(/<meta name="description" content="[^"]+">/)
            // Should not be a translation key
            expect(html).not.toContain('content="seo.')
        })

        it('should have robots meta tag', () => {
            expect(html).toContain('<meta name="robots" content="index,follow">')
        })

        it('should have Open Graph title', () => {
            expect(html).toMatch(/<meta property="og:title" content="[^"]+">/)
        })

        it('should have Open Graph description', () => {
            expect(html).toMatch(/<meta property="og:description" content="[^"]+">/)
        })

        it('should have Open Graph image', () => {
            expect(html).toMatch(/<meta property="og:image" content="[^"]+Slothlike-og\.jpg">/)
        })

        it('should have Open Graph locale', () => {
            expect(html).toContain('<meta property="og:locale" content="en_GB">')
        })

        it('should have Twitter card tags', () => {
            expect(html).toContain('<meta property="twitter:card" content="summary_large_image">')
            expect(html).toMatch(/<meta property="twitter:title" content="[^"]+">/)
            expect(html).toMatch(
                /<meta property="twitter:image" content="[^"]+Slothlike-twitter\.jpg">/
            )
        })

        it('should have canonical link', () => {
            expect(html).toMatch(/<link rel="canonical" href="[^"]+\/en"/)
        })

        it('should have hreflang tags', () => {
            expect(html).toMatch(/<link rel="alternate" href="[^"]+" hreflang="en">/)
            expect(html).toMatch(/<link rel="alternate" href="[^"]+" hreflang="de">/)
            expect(html).toMatch(/<link rel="alternate" href="[^"]+" hreflang="x-default">/)
        })

        it('should have lang attribute on html element', () => {
            expect(html).toMatch(/<html[^>]+lang="en"/)
        })

        it('should have pre-rendered content (not empty body)', () => {
            // The body should have actual content, not just an empty div
            // A properly SSG'd page should be larger than 10KB
            expect(html.length).toBeGreaterThan(10000)
            // Should contain actual page content like navigation
            expect(html).toContain('RDataCore')
        })
    })

    describe('SEO Meta Tags - German Home Page', () => {
        let html

        beforeAll(() => {
            html = readFileSync(join(distDir, 'de.html'), 'utf-8')
        })

        it('should have German locale in Open Graph', () => {
            expect(html).toContain('<meta property="og:locale" content="de_DE">')
        })

        it('should have lang="de" on html element', () => {
            expect(html).toMatch(/<html[^>]+lang="de"/)
        })

        it('should have canonical link for German page', () => {
            expect(html).toMatch(/<link rel="canonical" href="[^"]+\/de"/)
        })
    })

    describe('Privacy Page (noindex)', () => {
        let html

        beforeAll(() => {
            html = readFileSync(join(distDir, 'en/privacy.html'), 'utf-8')
        })

        it('should have noindex,nofollow robots meta tag', () => {
            expect(html).toContain('<meta name="robots" content="noindex,nofollow">')
        })
    })
})
