import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import FeatureCard from '../FeatureCard.vue'

describe('FeatureCard', () => {
    const defaultProps = {
        icon: 'database',
        title: 'Fast Performance',
        description: 'Optimized for speed',
    }

    it('should render the component', () => {
        const wrapper = mount(FeatureCard, {
            props: defaultProps,
        })
        expect(wrapper.exists()).toBe(true)
    })

    it('should display title', () => {
        const wrapper = mount(FeatureCard, {
            props: defaultProps,
        })
        expect(wrapper.text()).toContain('Fast Performance')
    })

    it('should display description', () => {
        const wrapper = mount(FeatureCard, {
            props: defaultProps,
        })
        expect(wrapper.text()).toContain('Optimized for speed')
    })

    it('should render icon', () => {
        const wrapper = mount(FeatureCard, {
            props: defaultProps,
        })
        const icon = wrapper.findComponent({ name: 'SmartIcon' })
        expect(icon.exists()).toBe(true)
    })

    it('should pass icon prop to SmartIcon', () => {
        const wrapper = mount(FeatureCard, {
            props: defaultProps,
        })
        const icon = wrapper.findComponent({ name: 'SmartIcon' })
        expect(icon.props('icon')).toBe('database')
    })

    it('should accept different icons', () => {
        const wrapper = mount(FeatureCard, {
            props: {
                ...defaultProps,
                icon: 'zap',
            },
        })
        const icon = wrapper.findComponent({ name: 'SmartIcon' })
        expect(icon.props('icon')).toBe('zap')
    })

    it('should render with minimal props', () => {
        const wrapper = mount(FeatureCard, {
            props: {
                icon: 'check',
                title: 'Title',
                description: 'Description',
            },
        })
        expect(wrapper.exists()).toBe(true)
    })
})
