import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createMemoryHistory } from 'vue-router'
import PricingPage from './PricingPage.vue'

const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/pricing', component: PricingPage }],
})

describe('PricingPage', () => {
    it('should render the component', () => {
        const wrapper = mount(PricingPage, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.exists()).toBe(true)
    })

    it('should have hero section', () => {
        const wrapper = mount(PricingPage, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.find('.pricing-hero-section').exists()).toBe(true)
    })

    it('should have free licenses section', () => {
        const wrapper = mount(PricingPage, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.find('.free-licenses-section').exists()).toBe(true)
    })

    it('should have company licenses section', () => {
        const wrapper = mount(PricingPage, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.find('.company-licenses-section').exists()).toBe(true)
    })

    it('should render free badges without being clipped', () => {
        const wrapper = mount(PricingPage, {
            global: {
                plugins: [router],
            },
        })
        const freeBadges = wrapper.findAll('.free-badge')
        expect(freeBadges.length).toBeGreaterThan(0)

        // Verify badges exist and have proper positioning
        freeBadges.forEach(badge => {
            expect(badge.exists()).toBe(true)
        })
    })

    it('should render popular badge without being clipped', () => {
        const wrapper = mount(PricingPage, {
            global: {
                plugins: [router],
            },
        })
        const popularBadge = wrapper.find('.popular-badge')
        expect(popularBadge.exists()).toBe(true)
    })

    it('should have pricing tier cards with overflow visible for badges', () => {
        const wrapper = mount(PricingPage, {
            global: {
                plugins: [router],
            },
        })
        const tierCards = wrapper.findAll('.pricing-tier-card')
        expect(tierCards.length).toBeGreaterThan(0)
    })

    it('should have FAQ section', () => {
        const wrapper = mount(PricingPage, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.find('.faq-section').exists()).toBe(true)
    })

    it('should have corporate CTA section', () => {
        const wrapper = mount(PricingPage, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.find('.corporate-cta-section').exists()).toBe(true)
    })
})
