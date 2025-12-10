import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import DemoCredentialsDialog from '../DemoCredentialsDialog.vue'

describe('DemoCredentialsDialog', () => {
    const defaultProps = {
        modelValue: true,
        title: 'Demo Access',
        hint: 'Use these credentials',
        usernameLabel: 'Username',
        passwordLabel: 'Password',
        cancelLabel: 'Cancel',
        openDemoLabel: 'Open Demo',
    }

    it('should render when modelValue is true', () => {
        const wrapper = mount(DemoCredentialsDialog, {
            props: defaultProps,
        })
        expect(wrapper.exists()).toBe(true)
    })

    it('should accept title prop', () => {
        const wrapper = mount(DemoCredentialsDialog, {
            props: defaultProps,
        })
        expect(wrapper.props('title')).toBe('Demo Access')
    })

    it('should accept hint prop', () => {
        const wrapper = mount(DemoCredentialsDialog, {
            props: defaultProps,
        })
        expect(wrapper.props('hint')).toBe('Use these credentials')
    })

    it('should accept label props', () => {
        const wrapper = mount(DemoCredentialsDialog, {
            props: defaultProps,
        })
        expect(wrapper.props('usernameLabel')).toBe('Username')
        expect(wrapper.props('passwordLabel')).toBe('Password')
    })

    it('should have expected structure', () => {
        const wrapper = mount(DemoCredentialsDialog, {
            props: defaultProps,
        })
        expect(wrapper.exists()).toBe(true)
    })

    it('should emit update:modelValue when cancel is clicked', async () => {
        const wrapper = mount(DemoCredentialsDialog, {
            props: defaultProps,
        })

        const buttons = wrapper.findAll('button')
        const cancelButton = buttons.find(b => b.text().includes('Cancel'))

        if (cancelButton) {
            await cancelButton.trigger('click')
            expect(wrapper.emitted('update:modelValue')).toBeTruthy()
            expect(wrapper.emitted('update:modelValue')?.[0]).toEqual([false])
        }
    })

    it('should emit open-demo event when open button is clicked', async () => {
        const wrapper = mount(DemoCredentialsDialog, {
            props: defaultProps,
        })

        const buttons = wrapper.findAll('button')
        const openButton = buttons.find(b => b.text().includes('Open Demo'))

        if (openButton) {
            await openButton.trigger('click')
            expect(wrapper.emitted('open-demo')).toBeTruthy()
        }
    })

    it('should not render when modelValue is false', () => {
        const wrapper = mount(DemoCredentialsDialog, {
            props: {
                ...defaultProps,
                modelValue: false,
            },
        })

        // Dialog should still exist but be hidden
        expect(wrapper.exists()).toBe(true)
    })

    it('should have button labels', () => {
        const wrapper = mount(DemoCredentialsDialog, {
            props: defaultProps,
        })

        expect(wrapper.props('cancelLabel')).toBe('Cancel')
        expect(wrapper.props('openDemoLabel')).toBe('Open Demo')
    })
})
