import { describe, it, expect, beforeEach, vi } from 'vitest'
import { useSEO } from './useSEO'
import { mount } from '@vue/test-utils'
import { createMemoryHistory, createRouter } from 'vue-router'
import { defineComponent, h } from 'vue'

// Mock the env-check module
vi.mock('@/env-check', () => ({
    env: {
        siteName: 'RDataCore',
    },
    getBaseUrl: () => 'https://rdatacore.test',
}))

// Mock the translations composable
vi.mock('./useTranslations', () => ({
    useTranslations: () => ({
        currentLanguage: { value: 'en' },
    }),
}))

const TestComponent = defineComponent({
    setup() {
        useSEO({
            title: 'Test Page',
            description: 'Test description',
            keywords: ['test', 'page'],
        })
        return () => h('div', 'Test')
    },
})

describe('useSEO', () => {
    beforeEach(() => {
        // Clear head elements
        document.head.innerHTML = ''
        // Reset title
        document.title = ''
    })

    const createTestRouter = () => {
        return createRouter({
            history: createMemoryHistory(),
            routes: [
                { path: '/', component: TestComponent },
                { path: '/about', component: TestComponent },
                { path: '/privacy', component: TestComponent, meta: { noIndex: true } },
            ],
        })
    }

    it('should set document title', async () => {
        const router = createTestRouter()
        await router.push('/')
        await router.isReady()

        mount(TestComponent, {
            global: { plugins: [router] },
        })

        expect(document.title).toBe('Test Page · RDataCore')
    })

    it('should set meta description', async () => {
        const router = createTestRouter()
        await router.push('/')
        await router.isReady()

        mount(TestComponent, {
            global: { plugins: [router] },
        })

        const metaDesc = document.querySelector('meta[name="description"]')
        expect(metaDesc?.getAttribute('content')).toBe('Test description')
    })

    it('should set robots meta tag to index,follow by default', async () => {
        const router = createTestRouter()
        await router.push('/')
        await router.isReady()

        mount(TestComponent, {
            global: { plugins: [router] },
        })

        const robots = document.querySelector('meta[name="robots"]')
        expect(robots?.getAttribute('content')).toBe('index,follow')
    })

    it('should set robots meta tag to noindex,nofollow for noIndex pages', async () => {
        const router = createTestRouter()
        await router.push('/privacy')
        await router.isReady()

        mount(TestComponent, {
            global: { plugins: [router] },
        })

        const robots = document.querySelector('meta[name="robots"]')
        expect(robots?.getAttribute('content')).toBe('noindex,nofollow')
    })

    it('should set canonical link', async () => {
        const router = createTestRouter()
        await router.push('/')
        await router.isReady()

        mount(TestComponent, {
            global: { plugins: [router] },
        })

        const canonical = document.querySelector('link[rel="canonical"]')
        expect(canonical?.getAttribute('href')).toBe('https://rdatacore.test/')
    })

    it('should set Open Graph tags', async () => {
        const router = createTestRouter()
        await router.push('/')
        await router.isReady()

        mount(TestComponent, {
            global: { plugins: [router] },
        })

        const ogTitle = document.querySelector('meta[property="og:title"]')
        expect(ogTitle?.getAttribute('content')).toBe('Test Page · RDataCore')

        const ogDesc = document.querySelector('meta[property="og:description"]')
        expect(ogDesc?.getAttribute('content')).toBe('Test description')

        const ogUrl = document.querySelector('meta[property="og:url"]')
        expect(ogUrl?.getAttribute('content')).toBe('https://rdatacore.test/')

        const ogSiteName = document.querySelector('meta[property="og:site_name"]')
        expect(ogSiteName?.getAttribute('content')).toBe('RDataCore')

        const ogLocale = document.querySelector('meta[property="og:locale"]')
        expect(ogLocale?.getAttribute('content')).toBe('en_US')

        const ogType = document.querySelector('meta[property="og:type"]')
        expect(ogType?.getAttribute('content')).toBe('website')

        const ogImage = document.querySelector('meta[property="og:image"]')
        expect(ogImage?.getAttribute('content')).toBe(
            'https://rdatacore.test/images/Slothlike-og.jpg'
        )
    })

    it('should set Twitter Card tags', async () => {
        const router = createTestRouter()
        await router.push('/')
        await router.isReady()

        mount(TestComponent, {
            global: { plugins: [router] },
        })

        const twitterCard = document.querySelector('meta[property="twitter:card"]')
        expect(twitterCard?.getAttribute('content')).toBe('summary_large_image')

        const twitterTitle = document.querySelector('meta[property="twitter:title"]')
        expect(twitterTitle?.getAttribute('content')).toBe('Test Page · RDataCore')

        const twitterDesc = document.querySelector('meta[property="twitter:description"]')
        expect(twitterDesc?.getAttribute('content')).toBe('Test description')

        const twitterImage = document.querySelector('meta[property="twitter:image"]')
        expect(twitterImage?.getAttribute('content')).toBe(
            'https://rdatacore.test/images/Slothlike-twitter.jpg'
        )
    })

    it('should set hreflang tags for all languages', async () => {
        const router = createTestRouter()
        await router.push('/')
        await router.isReady()

        mount(TestComponent, {
            global: { plugins: [router] },
        })

        const hreflangEn = document.querySelector('link[rel="alternate"][hreflang="en"]')
        expect(hreflangEn?.getAttribute('href')).toBe('https://rdatacore.test/en')

        const hreflangDe = document.querySelector('link[rel="alternate"][hreflang="de"]')
        expect(hreflangDe?.getAttribute('href')).toBe('https://rdatacore.test/de')

        const hreflangDefault = document.querySelector(
            'link[rel="alternate"][hreflang="x-default"]'
        )
        expect(hreflangDefault?.getAttribute('href')).toBe('https://rdatacore.test/')
    })

    it('should set keywords when provided', async () => {
        const router = createTestRouter()
        await router.push('/')
        await router.isReady()

        mount(TestComponent, {
            global: { plugins: [router] },
        })

        const keywords = document.querySelector('meta[name="keywords"]')
        expect(keywords?.getAttribute('content')).toBe('test, page')
    })

    it('should handle locale option correctly', async () => {
        const router = createTestRouter()
        await router.push('/')
        await router.isReady()

        const TestComponentWithLocale = defineComponent({
            setup() {
                useSEO({
                    title: 'Test Page',
                    description: 'Test description',
                    locale: 'de',
                })
                return () => h('div', 'Test')
            },
        })

        mount(TestComponentWithLocale, {
            global: { plugins: [router] },
        })

        const ogLocale = document.querySelector('meta[property="og:locale"]')
        // Should use currentLanguage from composable, not the optional locale
        expect(ogLocale?.getAttribute('content')).toBe('en_US')
    })
})
