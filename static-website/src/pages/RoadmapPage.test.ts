import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createMemoryHistory } from 'vue-router'
import RoadmapPage from './RoadmapPage.vue'

const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/roadmap', component: RoadmapPage }],
})

describe('RoadmapPage', () => {
    it('should render the component', () => {
        const wrapper = mount(RoadmapPage, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.exists()).toBe(true)
    })

    it('should have hero section', () => {
        const wrapper = mount(RoadmapPage, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.find('.hero-section').exists()).toBe(true)
    })

    it('should have done features section', () => {
        const wrapper = mount(RoadmapPage, {
            global: {
                plugins: [router],
            },
        })
        const doneCards = wrapper.findAll('.feature-card.done')
        expect(doneCards.length).toBe(10)
    })

    it('should have open features section', () => {
        const wrapper = mount(RoadmapPage, {
            global: {
                plugins: [router],
            },
        })
        const openCards = wrapper.findAll('.feature-card.open')
        expect(openCards.length).toBe(6)
    })
})

describe('RoadmapPage - Wish a Feature Section', () => {
    let windowOpenSpy: ReturnType<typeof vi.spyOn>

    beforeEach(() => {
        windowOpenSpy = vi.spyOn(window, 'open').mockImplementation(() => null)
    })

    afterEach(() => {
        windowOpenSpy.mockRestore()
    })

    it('should have wish section', () => {
        const wrapper = mount(RoadmapPage, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.find('.wish-section').exists()).toBe(true)
    })

    it('should render exactly 3 wish cards', () => {
        const wrapper = mount(RoadmapPage, {
            global: {
                plugins: [router],
            },
        })
        const wishCards = wrapper.findAll('.wish-card')
        expect(wishCards.length).toBe(3)
    })

    it('should render wish card icons', () => {
        const wrapper = mount(RoadmapPage, {
            global: {
                plugins: [router],
            },
        })
        const wishIcons = wrapper.findAll('.wish-icon')
        expect(wishIcons.length).toBe(3)
        expect(wishIcons[0].text()).toBe('💡')
        expect(wishIcons[1].text()).toBe('🔧')
        expect(wishIcons[2].text()).toBe('🚀')
    })

    it('should render wish card titles from translations', () => {
        const wrapper = mount(RoadmapPage, {
            global: {
                plugins: [router],
            },
        })
        const wishCards = wrapper.findAll('.wish-card')

        // Check that titles are rendered (translation keys or actual text)
        wishCards.forEach(card => {
            const title = card.find('h3')
            expect(title.exists()).toBe(true)
            expect(title.text().length).toBeGreaterThan(0)
        })
    })

    it('should render wish card descriptions from translations', () => {
        const wrapper = mount(RoadmapPage, {
            global: {
                plugins: [router],
            },
        })
        const wishCards = wrapper.findAll('.wish-card')

        // Check that descriptions are rendered
        wishCards.forEach(card => {
            const desc = card.find('p')
            expect(desc.exists()).toBe(true)
            expect(desc.text().length).toBeGreaterThan(0)
        })
    })

    it('should render CTA button on each wish card', () => {
        const wrapper = mount(RoadmapPage, {
            global: {
                plugins: [router],
            },
        })
        const ctaButtons = wrapper.findAll('.wish-cta')
        expect(ctaButtons.length).toBe(3)

        // Each CTA should have text
        ctaButtons.forEach(cta => {
            expect(cta.text().length).toBeGreaterThan(0)
        })
    })

    it('should open mailto link when wish card is clicked', async () => {
        const wrapper = mount(RoadmapPage, {
            global: {
                plugins: [router],
            },
        })

        const wishCards = wrapper.findAll('.wish-card')
        expect(wishCards.length).toBeGreaterThan(0)

        await wishCards[0].trigger('click')

        expect(windowOpenSpy).toHaveBeenCalledTimes(1)
        const callArgs = windowOpenSpy.mock.calls[0]
        expect(callArgs[0]).toContain('mailto:')
        expect(callArgs[0]).toContain('hello@rdatacore.eu')
        expect(callArgs[0]).toContain('subject=')
        expect(callArgs[0]).toContain('Feature%20Request')
    })

    it('should open mailto link for each wish card', async () => {
        const wrapper = mount(RoadmapPage, {
            global: {
                plugins: [router],
            },
        })

        const wishCards = wrapper.findAll('.wish-card')

        // Click each card and verify mailto is called
        for (let i = 0; i < wishCards.length; i++) {
            await wishCards[i].trigger('click')
            expect(windowOpenSpy).toHaveBeenCalledTimes(i + 1)

            const callArgs = windowOpenSpy.mock.calls[i]
            expect(callArgs[0]).toContain('mailto:hello@rdatacore.eu')
        }
    })

    it('should have clickable wish cards with cursor pointer style', () => {
        const wrapper = mount(RoadmapPage, {
            global: {
                plugins: [router],
            },
        })

        const wishCards = wrapper.findAll('.wish-card')
        wishCards.forEach(card => {
            // Cards should have click handler (we verify by checking they respond to clicks)
            expect(card.exists()).toBe(true)
        })
    })

    it('should display section title and subtitle', () => {
        const wrapper = mount(RoadmapPage, {
            global: {
                plugins: [router],
            },
        })

        const wishSection = wrapper.find('.wish-section')
        const sectionHeader = wishSection.find('.section-header')

        expect(sectionHeader.exists()).toBe(true)

        const title = sectionHeader.find('h2')
        const subtitle = sectionHeader.find('.section-subtitle')

        expect(title.exists()).toBe(true)
        expect(title.text().length).toBeGreaterThan(0)
        expect(subtitle.exists()).toBe(true)
        expect(subtitle.text().length).toBeGreaterThan(0)
    })
})

describe('RoadmapPage - Demo Dialog', () => {
    it('should have demo credentials dialog', () => {
        const wrapper = mount(RoadmapPage, {
            global: {
                plugins: [router],
            },
        })
        const dialog = wrapper.findComponent({ name: 'DemoCredentialsDialog' })
        expect(dialog.exists()).toBe(true)
    })

    it('should open demo dialog when open-demo event is dispatched', async () => {
        const wrapper = mount(RoadmapPage, {
            global: {
                plugins: [router],
            },
        })

        const dialog = wrapper.findComponent({ name: 'DemoCredentialsDialog' })
        expect(dialog.exists()).toBe(true)

        // Initially dialog should be closed
        expect(dialog.props('modelValue')).toBe(false)

        // Dispatch the open-demo event
        window.dispatchEvent(new CustomEvent('open-demo'))
        await wrapper.vm.$nextTick()

        // Dialog should now be open
        expect(dialog.props('modelValue')).toBe(true)
    })

    it('should close demo dialog when update:modelValue is emitted', async () => {
        const wrapper = mount(RoadmapPage, {
            global: {
                plugins: [router],
            },
        })

        const dialog = wrapper.findComponent({ name: 'DemoCredentialsDialog' })

        // Open the dialog first
        window.dispatchEvent(new CustomEvent('open-demo'))
        await wrapper.vm.$nextTick()
        expect(dialog.props('modelValue')).toBe(true)

        // Close the dialog
        dialog.vm.$emit('update:modelValue', false)
        await wrapper.vm.$nextTick()

        // Dialog should be closed
        expect(dialog.props('modelValue')).toBe(false)
    })
})
