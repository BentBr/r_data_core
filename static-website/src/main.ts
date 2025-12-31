// SSR polyfills - must be first before any Vuetify imports
// These polyfill browser APIs that aren't available in Node.js/jsdom
if (typeof globalThis.ResizeObserver === 'undefined') {
    globalThis.ResizeObserver = class ResizeObserver {
        observe() {}
        unobserve() {}
        disconnect() {}
    } as unknown as typeof ResizeObserver
}

// Polyfill window.scrollTo for SSR (jsdom throws "Not implemented" error)
if (typeof window !== 'undefined') {
    window.scrollTo = () => {}
}

import { ViteSSG } from 'vite-ssg'
import { createVuetify } from 'vuetify'
// Import only the components we use for tree-shaking
import {
    VAlert,
    VApp,
    VBtn,
    VCard,
    VCardActions,
    VCardText,
    VCardTitle,
    VCol,
    VContainer,
    VDialog,
    VDivider,
    VIcon,
    VList,
    VListItem,
    VListItemTitle,
    VMenu,
    VNavigationDrawer,
    VOverlay,
    VRow,
} from 'vuetify/components'
import App from './App.vue'
import { vuetifyDefaults, vuetifyTheme } from './design-system'
import SmartIcon from './components/common/SmartIcon.vue'
import type { IconAliases } from 'vuetify'
import { checkEnvironmentVariables } from './env-check'
import { routes } from './router'
import { useTranslations } from './composables/useTranslations'

import 'vuetify/styles'

const iconAliases: Partial<IconAliases> = {
    collapse: 'chevron-up',
    complete: 'check',
    cancel: 'x',
    close: 'x',
    delete: 'trash-2',
    clear: 'x-circle',
    success: 'check-circle',
    info: 'info',
    warning: 'alert-triangle',
    error: 'alert-octagon',
    prev: 'chevron-left',
    next: 'chevron-right',
    first: 'chevrons-left',
    last: 'chevrons-right',
    delimiter: 'circle',
    sort: 'arrow-up-down',
    sortAsc: 'arrow-up',
    sortDesc: 'arrow-down',
    expand: 'chevron-down',
    menu: 'menu',
    subgroup: 'chevron-down',
    dropdown: 'chevron-down',
    menuRight: 'chevron-right',
    menuDown: 'chevron-down',
    menuLeft: 'chevron-left',
    menuUp: 'chevron-up',
    radioOn: 'dot',
    radioOff: 'circle',
    edit: 'pencil',
    ratingEmpty: 'star',
    ratingFull: 'star',
    ratingHalf: 'star-half',
    checkboxOn: 'check-square',
    checkboxOff: 'square',
    checkboxIndeterminate: 'minus-square',
}

// Log environment variables in dev for sanity
if (import.meta.env.DEV) {
    checkEnvironmentVariables()
}

// https://github.com/antfu/vite-ssg
export const createApp = ViteSSG(
    App,
    {
        routes,
        base: import.meta.env.BASE_URL,
        // Scroll behavior only runs on client - return false during SSR to avoid errors
        scrollBehavior(to, _from, savedPosition) {
            // Skip scroll during SSR (window.scrollTo not implemented in jsdom)
            if (typeof window === 'undefined' || !window.scrollTo) {
                return false
            }
            if (savedPosition) {
                return savedPosition
            } else if (to.hash) {
                return { el: to.hash, behavior: 'smooth' }
            } else {
                return { top: 0, behavior: 'smooth' }
            }
        },
    },
    async ({ app, router, isClient }) => {
        const vuetify = createVuetify({
            ssr: true,
            components: {
                VAlert,
                VApp,
                VBtn,
                VCard,
                VCardActions,
                VCardText,
                VCardTitle,
                VCol,
                VContainer,
                VDialog,
                VDivider,
                VIcon,
                VList,
                VListItem,
                VListItemTitle,
                VMenu,
                VNavigationDrawer,
                VOverlay,
                VRow,
            },
            theme: vuetifyTheme,
            defaults: vuetifyDefaults,
            icons: {
                defaultSet: 'smart',
                aliases: iconAliases,
                sets: {
                    smart: {
                        // Type assertion needed due to Vuetify IconComponent type mismatch
                        // eslint-disable-next-line @typescript-eslint/no-explicit-any
                        component: SmartIcon as unknown as any,
                    },
                },
            },
        })

        app.use(vuetify)

        // Set up router guard for language
        router.beforeEach((to, _from, next) => {
            // Allow static files to pass through (client only)
            if (isClient) {
                const staticPaths = [
                    '/sitemap.xml',
                    '/sitemap_en.xml',
                    '/sitemap_de.xml',
                    '/robots.txt',
                ]

                if (staticPaths.includes(to.path)) {
                    window.location.href = to.path
                    return
                }
            }

            // Set language based on route (both client and server)
            const lang = (to.params.lang as string) || 'en'
            const { setLanguage } = useTranslations()

            if (lang === 'de') {
                setLanguage('de')
            } else {
                setLanguage('en')
            }
            next()
        })
    }
)
