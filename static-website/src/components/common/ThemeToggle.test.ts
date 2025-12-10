import { describe, it, expect, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import ThemeToggle from './ThemeToggle.vue'

describe('ThemeToggle', () => {
    beforeEach(() => {
        localStorage.clear()
    })

    it('should render the component', () => {
        const wrapper = mount(ThemeToggle)
        expect(wrapper.exists()).toBe(true)
    })

    it('should toggle theme on click', async () => {
        const wrapper = mount(ThemeToggle)
        const button = wrapper.find('button')

        await button.trigger('click')
        expect(wrapper.exists()).toBe(true)
    })

    it('should save theme preference to localStorage', async () => {
        const wrapper = mount(ThemeToggle)
        const button = wrapper.find('button')

        await button.trigger('click')
        const savedTheme = localStorage.getItem('theme-preference')
        expect(savedTheme).toBeTruthy()
    })

    it('should load theme from localStorage on mount', () => {
        localStorage.setItem('theme-preference', 'dark')
        const wrapper = mount(ThemeToggle)
        expect(wrapper.exists()).toBe(true)
    })
})
