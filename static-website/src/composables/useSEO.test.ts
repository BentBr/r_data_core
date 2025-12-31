import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createMemoryHistory, createRouter } from 'vue-router'
import { defineComponent, h, ref, ComputedRef } from 'vue'

// Track what useHead receives - must be defined before mock
const headDataCapture: { value: ComputedRef | null } = { value: null }

// Mock @vueuse/head - must use inline function to avoid hoisting issues
vi.mock('@vueuse/head', () => ({
    useHead: vi.fn((data: ComputedRef) => {
        // Access captured data through an object reference
        headDataCapture.value = data
    }),
}))

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
        currentLanguage: ref('en'),
    }),
}))

// Import after mocks are set up
import { useSEO } from './useSEO'
import { useHead } from '@vueuse/head'

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
    const createTestRouter = (
        routes = [
            { path: '/', component: TestComponent },
            { path: '/about', component: TestComponent },
            { path: '/privacy', component: TestComponent, meta: { noIndex: true } },
        ]
    ) => {
        return createRouter({
            history: createMemoryHistory(),
            routes,
        })
    }

    beforeEach(() => {
        vi.clearAllMocks()
        headDataCapture.value = null
    })

    it('should call useHead with correct title', async () => {
        const router = createTestRouter()
        await router.push('/')
        await router.isReady()

        mount(TestComponent, {
            global: { plugins: [router] },
        })

        expect(useHead).toHaveBeenCalled()
        expect(headDataCapture.value).toBeTruthy()

        const headValue = headDataCapture.value!.value
        expect(headValue.title).toBe('Test Page · RDataCore')
    })

    it('should set meta description', async () => {
        const router = createTestRouter()
        await router.push('/')
        await router.isReady()

        mount(TestComponent, {
            global: { plugins: [router] },
        })

        const headValue = headDataCapture.value!.value
        const descMeta = headValue.meta.find((m: { name?: string }) => m.name === 'description')
        expect(descMeta?.content).toBe('Test description')
    })

    it('should set robots meta tag to index,follow by default', async () => {
        const router = createTestRouter()
        await router.push('/')
        await router.isReady()

        mount(TestComponent, {
            global: { plugins: [router] },
        })

        const headValue = headDataCapture.value!.value
        const robotsMeta = headValue.meta.find((m: { name?: string }) => m.name === 'robots')
        expect(robotsMeta?.content).toBe('index,follow')
    })

    it('should set robots meta tag to noindex,nofollow for noIndex pages', async () => {
        const router = createTestRouter()
        await router.push('/privacy')
        await router.isReady()

        mount(TestComponent, {
            global: { plugins: [router] },
        })

        const headValue = headDataCapture.value!.value
        const robotsMeta = headValue.meta.find((m: { name?: string }) => m.name === 'robots')
        expect(robotsMeta?.content).toBe('noindex,nofollow')
    })

    it('should set canonical link', async () => {
        const router = createTestRouter()
        await router.push('/')
        await router.isReady()

        mount(TestComponent, {
            global: { plugins: [router] },
        })

        const headValue = headDataCapture.value!.value
        const canonicalLink = headValue.link.find((l: { rel?: string }) => l.rel === 'canonical')
        expect(canonicalLink?.href).toBe('https://rdatacore.test/')
    })

    it('should set Open Graph tags', async () => {
        const router = createTestRouter()
        await router.push('/')
        await router.isReady()

        mount(TestComponent, {
            global: { plugins: [router] },
        })

        const headValue = headDataCapture.value!.value
        const findOgMeta = (prop: string) =>
            headValue.meta.find((m: { property?: string }) => m.property === prop)

        expect(findOgMeta('og:title')?.content).toBe('Test Page · RDataCore')
        expect(findOgMeta('og:description')?.content).toBe('Test description')
        expect(findOgMeta('og:url')?.content).toBe('https://rdatacore.test/')
        expect(findOgMeta('og:site_name')?.content).toBe('RDataCore')
        expect(findOgMeta('og:locale')?.content).toBe('en_GB')
        expect(findOgMeta('og:type')?.content).toBe('website')
        expect(findOgMeta('og:image')?.content).toBe(
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

        const headValue = headDataCapture.value!.value
        const findTwitterMeta = (prop: string) =>
            headValue.meta.find((m: { property?: string }) => m.property === prop)

        expect(findTwitterMeta('twitter:card')?.content).toBe('summary_large_image')
        expect(findTwitterMeta('twitter:title')?.content).toBe('Test Page · RDataCore')
        expect(findTwitterMeta('twitter:description')?.content).toBe('Test description')
        expect(findTwitterMeta('twitter:image')?.content).toBe(
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

        const headValue = headDataCapture.value!.value
        const findHreflang = (lang: string) =>
            headValue.link.find(
                (l: { rel?: string; hreflang?: string }) =>
                    l.rel === 'alternate' && l.hreflang === lang
            )

        expect(findHreflang('en')?.href).toBe('https://rdatacore.test/en')
        expect(findHreflang('de')?.href).toBe('https://rdatacore.test/de')
        expect(findHreflang('x-default')?.href).toBe('https://rdatacore.test/')
    })

    it('should set keywords when provided', async () => {
        const router = createTestRouter()
        await router.push('/')
        await router.isReady()

        mount(TestComponent, {
            global: { plugins: [router] },
        })

        const headValue = headDataCapture.value!.value
        const keywordsMeta = headValue.meta.find((m: { name?: string }) => m.name === 'keywords')
        expect(keywordsMeta?.content).toBe('test, page')
    })

    it('should set html lang attribute', async () => {
        const router = createTestRouter()
        await router.push('/')
        await router.isReady()

        mount(TestComponent, {
            global: { plugins: [router] },
        })

        const headValue = headDataCapture.value!.value
        expect(headValue.htmlAttrs.lang).toBe('en')
    })
})
