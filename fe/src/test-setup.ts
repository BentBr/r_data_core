/**
 * Vitest global test setup
 */
import { vi } from 'vitest'
import { defineComponent, h } from 'vue'
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
// `teleport: true` is Vue Test Utils' boolean stub, which renders the element
// but *drops its children*. Every v-dialog and v-menu body therefore never
// executed under test — v-dialog content came out as the literal string
// "[object Object]" — so those template branches were invisible to both
// assertions and coverage. That hid a MainLayout test mounting the wrong
// component entirely, kept several dialogs stuck at low coverage, and let a
// real form bug ship because the <v-form> inside a dialog never rendered.
//
// This stub renders the slot in place instead. Branches execute, and unlike
// `teleport: false` the content stays inside the wrapper, so `wrapper.find()`
// keeps working and no node leaks into document.body between tests.
//
// A render function rather than a `template`, so this does not require the
// runtime template compiler.
config.global.stubs = {
    teleport: defineComponent({
        name: 'TeleportStub',
        setup(_props, { slots }) {
            return () => h('div', { 'data-teleport-stub': '' }, slots.default?.())
        },
    }),
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
global.console = { ...console, warn: vi.fn(), info: vi.fn(), error: vi.fn() }

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
