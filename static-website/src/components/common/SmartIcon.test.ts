import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import SmartIcon from './SmartIcon.vue'

describe('SmartIcon', () => {
    it('should render the component', () => {
        const wrapper = mount(SmartIcon, {
            props: {
                icon: 'database',
            },
        })
        expect(wrapper.exists()).toBe(true)
    })

    it('should render LucideIcon', () => {
        const wrapper = mount(SmartIcon, {
            props: {
                icon: 'database',
            },
        })
        const lucideIcon = wrapper.findComponent({ name: 'LucideIcon' })
        expect(lucideIcon.exists()).toBe(true)
    })

    it('should pass icon prop to LucideIcon', () => {
        const wrapper = mount(SmartIcon, {
            props: {
                icon: 'database',
            },
        })
        const lucideIcon = wrapper.findComponent({ name: 'LucideIcon' })
        expect(lucideIcon.props('name')).toBe('database')
    })

    it('should pass size prop', () => {
        const wrapper = mount(SmartIcon, {
            props: {
                icon: 'database',
                size: 'lg',
            },
        })
        const lucideIcon = wrapper.findComponent({ name: 'LucideIcon' })
        expect(lucideIcon.props('size')).toBeDefined()
    })

    it('should pass color prop', () => {
        const wrapper = mount(SmartIcon, {
            props: {
                icon: 'database',
                color: 'primary',
            },
        })
        const lucideIcon = wrapper.findComponent({ name: 'LucideIcon' })
        expect(lucideIcon.props('color')).toBe('primary')
    })

    it('should handle different icon names', () => {
        const icons = ['database', 'zap', 'check', 'heart', 'code-2']

        icons.forEach(iconName => {
            const wrapper = mount(SmartIcon, {
                props: {
                    icon: iconName,
                },
            })
            expect(wrapper.exists()).toBe(true)
        })
    })

    it('should accept size as number', () => {
        const wrapper = mount(SmartIcon, {
            props: {
                icon: 'database',
                size: 32,
            },
        })
        expect(wrapper.exists()).toBe(true)
    })

    it('should accept size as string', () => {
        const wrapper = mount(SmartIcon, {
            props: {
                icon: 'database',
                size: 'xl',
            },
        })
        expect(wrapper.exists()).toBe(true)
    })
})
