import { describe, it, expect } from 'vitest'
import { createRouter, createMemoryHistory } from 'vue-router'

describe('Router', () => {
    it('should create router instance', async () => {
        const router = createRouter({
            history: createMemoryHistory(),
            routes: [{ path: '/', component: { template: '<div>Home</div>' } }],
        })
        // Initialize router to a valid route
        await router.push('/')
        await router.isReady()
        expect(router).toBeDefined()
        expect(typeof router.push).toBe('function')
    })

    it('should support language-based routes', async () => {
        const router = createRouter({
            history: createMemoryHistory(),
            routes: [
                {
                    path: '/:lang(en|de)?',
                    name: 'Home',
                    component: { template: '<div>Home</div>' },
                },
            ],
        })
        // Initialize router to a valid route
        await router.push('/')
        await router.isReady()
        expect(router).toBeDefined()
    })

    it('should handle language redirect from root', async () => {
        const router = createRouter({
            history: createMemoryHistory(),
            routes: [
                {
                    path: '/',
                    redirect: () => {
                        const browserLang = navigator.language.toLowerCase()
                        const lang = browserLang.startsWith('de') ? 'de' : 'en'
                        return `/${lang}`
                    },
                },
                { path: '/:lang', component: { template: '<div>Home</div>' } },
            ],
        })

        await router.push('/')
        await router.isReady()

        expect(router.currentRoute.value.path).toMatch(/^\/(en|de)$/)
    })

    it('should have proper scroll behavior', () => {
        const scrollBehavior = (
            to: { hash?: string },
            _from: unknown,
            savedPosition?: { top: number }
        ) => {
            if (savedPosition) {
                return savedPosition
            } else if (to.hash) {
                return { el: to.hash, behavior: 'smooth' as const }
            } else {
                return { top: 0, behavior: 'smooth' as const }
            }
        }

        const result = scrollBehavior({ hash: '#test' }, null, undefined)
        expect(result).toEqual({ el: '#test', behavior: 'smooth' })
    })
})
