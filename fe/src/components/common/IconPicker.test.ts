import { mount } from '@vue/test-utils'
import { describe, it, expect } from 'vitest'
import IconPicker from './IconPicker.vue'
import { createVuetify } from 'vuetify'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'

// Create Vuetify instance for testing
const vuetify = createVuetify({
    components,
    directives,
})

// IconPicker renders 500+ Lucide icon buttons — mounting is slow in CI/Docker
describe('IconPicker', { timeout: 30_000 }, () => {
    it('renders correctly', async () => {
        const wrapper = mount(IconPicker, {
            props: {
                modelValue: '',
                label: 'Select Icon',
            },
            global: {
                plugins: [vuetify],
            },
        })

        await wrapper.vm.$nextTick()

        expect(wrapper.find('input').exists()).toBe(true)
        expect(wrapper.find('.v-card').exists()).toBe(true)
    })

    it('emits update:modelValue when icon is selected', async () => {
        const wrapper = mount(IconPicker, {
            props: {
                modelValue: '',
                label: 'Select Icon',
            },
            global: {
                plugins: [vuetify],
            },
        })

        await wrapper.vm.$nextTick()

        const iconButton = wrapper.find('.v-btn')
        expect(iconButton.exists()).toBe(true)
        await iconButton.trigger('click')
        await wrapper.vm.$nextTick()

        expect(wrapper.emitted('update:modelValue')).toBeTruthy()
        expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['file'])
    })

    it('filters icons based on search query', async () => {
        const wrapper = mount(IconPicker, {
            props: {
                modelValue: '',
                label: 'Select Icon',
            },
            global: {
                plugins: [vuetify],
            },
        })

        await wrapper.vm.$nextTick()

        const searchInput = wrapper.find('input')
        await searchInput.setValue('folder')
        await wrapper.vm.$nextTick()

        // Should show filtered results
        const iconButtons = wrapper.findAll('.v-btn')
        expect(iconButtons.length).toBeGreaterThan(0)
    })
})
