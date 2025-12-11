import { createRouter, createWebHistory, type RouteRecordRaw } from 'vue-router'
import HomePage from '@/pages/HomePage.vue'
import { useTranslations } from '@/composables/useTranslations'

const routes: RouteRecordRaw[] = [
    {
        path: '/:lang(en|de)',
        name: 'Home',
        component: HomePage,
    },
    {
        path: '/:lang(en|de)/about',
        name: 'About',
        component: () => import('@/pages/AboutPage.vue'),
    },
    {
        path: '/:lang(en|de)/pricing',
        name: 'Pricing',
        component: () => import('@/pages/PricingPage.vue'),
    },
    {
        path: '/:lang(en|de)/roadmap',
        name: 'Roadmap',
        component: () => import('@/pages/RoadmapPage.vue'),
    },
    {
        path: '/:lang(en|de)/use-cases',
        name: 'UseCases',
        component: () => import('@/pages/UseCasesPage.vue'),
    },
    {
        path: '/:lang(en|de)/imprint',
        name: 'Imprint',
        component: () => import('@/pages/ImprintPage.vue'),
        meta: { noIndex: true },
    },
    {
        path: '/:lang(en|de)/privacy',
        name: 'Privacy',
        component: () => import('@/pages/PrivacyPage.vue'),
        meta: { noIndex: true },
    },
    {
        path: '/',
        redirect: () => {
            // Detect browser language
            const browserLang = navigator.language.toLowerCase()
            const lang = browserLang.startsWith('de') ? 'de' : 'en'
            return `/${lang}`
        },
    },
    {
        path: '/:pathMatch(.*)*',
        redirect: '/',
    },
]

const router = createRouter({
    history: createWebHistory(),
    routes,
    scrollBehavior(to, _from, savedPosition) {
        if (savedPosition) {
            return savedPosition
        } else if (to.hash) {
            return { el: to.hash, behavior: 'smooth' }
        } else {
            return { top: 0, behavior: 'smooth' }
        }
    },
})

// Update language based on route parameter, default to EN
router.beforeEach((to, _from, next) => {
    // Allow static files (sitemaps, robots.txt) to pass through
    // These will be handled by the server (nginx) in production
    const staticPaths = ['/sitemap.xml', '/sitemap_en.xml', '/sitemap_de.xml', '/robots.txt']

    if (staticPaths.includes(to.path)) {
        // In development, these files won't exist via Vue Router
        // Skip Vue Router handling for these paths
        window.location.href = to.path
        return
    }

    const lang = (to.params.lang as string) || 'en' // Default to English
    const { setLanguage } = useTranslations()
    // Ensure we always set a language (en or de)
    if (lang === 'de') {
        void setLanguage('de')
    } else {
        void setLanguage('en') // Explicit default to English
    }
    next()
})

export default router
