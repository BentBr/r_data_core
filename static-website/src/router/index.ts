import type { RouteRecordRaw } from 'vue-router'
import HomePage from '@/pages/HomePage.vue'

// Routes array for vite-ssg
export const routes: RouteRecordRaw[] = [
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
        redirect: '/en',
    },
    {
        path: '/:pathMatch(.*)*',
        redirect: '/en',
    },
]

// Export default for backward compatibility with tests
export default { routes }
