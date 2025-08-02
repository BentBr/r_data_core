import { mount } from '@vue/test-utils'
import IconPicker from './IconPicker.vue'

describe('IconPicker', () => {
    it('renders correctly', () => {
        const wrapper = mount(IconPicker, {
            props: {
                modelValue: '',
                label: 'Select Icon',
            },
        })

        expect(wrapper.find('input').exists()).toBe(true)
        expect(wrapper.find('.v-card').exists()).toBe(true)
    })

    it('emits update:modelValue when icon is selected', async () => {
        const wrapper = mount(IconPicker, {
            props: {
                modelValue: '',
                label: 'Select Icon',
            },
        })

        const iconButton = wrapper.find('.v-btn')
        await iconButton.trigger('click')

        expect(wrapper.emitted('update:modelValue')).toBeTruthy()
        expect(wrapper.emitted('update:modelValue')[0]).toEqual(['mdi-file-document'])
    })

    it('filters icons based on search query', async () => {
        const wrapper = mount(IconPicker, {
            props: {
                modelValue: '',
                label: 'Select Icon',
            },
        })

        const searchInput = wrapper.find('input')
        await searchInput.setValue('folder')

        // Should show filtered results
        const iconButtons = wrapper.findAll('.v-btn')
        expect(iconButtons.length).toBeGreaterThan(0)
    })
})
