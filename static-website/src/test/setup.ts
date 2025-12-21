import { config } from '@vue/test-utils'
import { createVuetify } from 'vuetify'
import {
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
    VList,
    VListItem,
    VListItemTitle,
    VMenu,
    VNavigationDrawer,
    VRow,
} from 'vuetify/components'

// Create a minimal Vuetify instance for tests
const vuetify = createVuetify({
    components: {
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
        VList,
        VListItem,
        VListItemTitle,
        VMenu,
        VNavigationDrawer,
        VRow,
    },
})

// Make Vuetify available globally in tests
config.global.plugins = [vuetify]

// Mock window.matchMedia
Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: (query: string) => ({
        matches: false,
        media: query,
        onchange: null,
        addListener: () => {},
        removeListener: () => {},
        addEventListener: () => {},
        removeEventListener: () => {},
        dispatchEvent: () => true,
    }),
})

// Mock IntersectionObserver
global.IntersectionObserver = class IntersectionObserver {
    constructor() {}
    disconnect() {}
    observe() {}
    takeRecords() {
        return []
    }
    unobserve() {}
} as unknown as typeof IntersectionObserver

// Mock visualViewport
Object.defineProperty(window, 'visualViewport', {
    writable: true,
    value: {
        width: 1024,
        height: 768,
        scale: 1,
        offsetLeft: 0,
        offsetTop: 0,
        pageLeft: 0,
        pageTop: 0,
        addEventListener: () => {},
        removeEventListener: () => {},
    },
})
