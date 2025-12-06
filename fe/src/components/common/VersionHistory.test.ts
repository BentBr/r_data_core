import { mount } from '@vue/test-utils'
import { describe, it, expect, vi } from 'vitest'
import VersionHistory from './VersionHistory.vue'
import { createVuetify } from 'vuetify'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'

// Mock translations
vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({
        t: (key: string) => key,
    }),
}))

const vuetify = createVuetify({
    components,
    directives,
})

const versions = [
    { version_number: 1, created_at: '2024-01-01T00:00:00Z' },
    { version_number: 2, created_at: '2024-01-02T00:00:00Z' },
    { version_number: 3, created_at: '2024-01-03T00:00:00Z' },
]

describe('VersionHistory', () => {
    it('emits compare when two versions are selected', async () => {
        const wrapper = mount(VersionHistory, {
            props: { versions },
            global: { plugins: [vuetify] },
        })

        const items = wrapper.findAllComponents({ name: 'VListItem' })
        expect(items.length).toBeGreaterThanOrEqual(2)

        await items[0].trigger('click')
        await items[1].trigger('click')

        const compareEvents = wrapper.emitted('compare')
        expect(compareEvents?.[0]).toEqual([1, 2])
    })

    it('renders diff table after updateDiffRows is called', async () => {
        const wrapper = mount(VersionHistory, {
            props: { versions },
            global: { plugins: [vuetify] },
        })

        // Select two versions to enable diff display
        const items = wrapper.findAllComponents({ name: 'VListItem' })
        await items[0].trigger('click')
        await items[1].trigger('click')
        ;(wrapper.vm as unknown as { updateDiffRows: (rows: unknown[]) => void }).updateDiffRows([
            { field: 'name', a: 'A', b: 'B', changed: true },
        ])

        await wrapper.vm.$nextTick()
        const rows = wrapper.findAll('tbody tr')
        expect(rows.length).toBe(1)
        expect(rows[0].text()).toContain('name')
    })
})
