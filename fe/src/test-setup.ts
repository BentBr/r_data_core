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
// This stub asks for the slot instead of discarding it, which makes the
// surrounding component's own render create those child vnodes — so its
// template lines execute and are measured. That is the win: UserProfileMenu
// went 58% -> 100% with no test change.
//
// What it does NOT do is give you a queryable DOM. Vue coerces a Teleport's
// children to a text vnode when they are re-created for a stub component, so
// the rendered output is the string "[object Object]". A test that asserts on
// markup *inside* a dialog or menu still needs `stubs: { teleport: false }`
// on that mount plus `new DOMWrapper(document.body)` to query it, and must
// unmount in afterEach so the teleported nodes do not leak into later tests.
// See DialogManager.test.ts for that pattern.
//
// `teleport: false` globally is not the answer: it moves every dialog body to
// document.body and breaks 24 existing tests that query through the wrapper.
//
// A render function rather than a `template`, so this needs no runtime
// compiler.
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
