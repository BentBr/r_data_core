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
    VList,
    VListItem,
    VListItemTitle,
    VMenu,
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
        VList,
        VListItem,
        VListItemTitle,
        VMenu,
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
