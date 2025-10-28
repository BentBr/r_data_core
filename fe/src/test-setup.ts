/**
 * Vitest global test setup
 */
import { vi } from 'vitest'
import { config } from '@vue/test-utils'
import { createVuetify } from 'vuetify'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'
import type { ComponentMountingOptions } from '@vue/test-utils'

// -----------------------------------------------------------------------------
// Vuetify
// -----------------------------------------------------------------------------
const vuetify = createVuetify({ components, directives })
config.global.plugins = [vuetify]
config.global.stubs = {
    teleport: true,
}

import { mount } from '@vue/test-utils'
export const mountX = (component: unknown, options: ComponentMountingOptions<unknown> = {}) =>
    mount(component, {
        ...options,
        global: { ...(options.global ?? {}), plugins: [vuetify] },
    })

// -----------------------------------------------------------------------------
// env
// -----------------------------------------------------------------------------
Object.defineProperty(globalThis, 'import', {
    value: { meta: { env: { DEV: true, PROD: false } } },
})

// -----------------------------------------------------------------------------
// ResizeObserver shim for Vuetify
class ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
}
;(global as typeof globalThis & { ResizeObserver: typeof ResizeObserver }).ResizeObserver =
    ResizeObserver

// translations helper
// -----------------------------------------------------------------------------
vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({ t: (k: string) => k.split('.').pop() }),
}))

// silence noisy console
global.console = { ...console, warn: vi.fn(), info: vi.fn() }

// Mock matchMedia for Vuetify components
Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: vi.fn().mockImplementation(query => ({
        matches: false,
        media: query,
        onchange: null,
        addListener: vi.fn(), // deprecated
        removeListener: vi.fn(), // deprecated
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        dispatchEvent: vi.fn(),
    })),
})

// Mock visualViewport for Vuetify components
Object.defineProperty(window, 'visualViewport', {
    writable: true,
    value: {
        width: 1024,
        height: 768,
        scale: 1,
        offsetLeft: 0,
        offsetTop: 0,
        onresize: null,
        onscroll: null,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
    },
})
