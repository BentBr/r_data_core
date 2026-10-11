import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { mount, DOMWrapper, type VueWrapper } from '@vue/test-utils'
import ApiKeyCreateDialog from './ApiKeyCreateDialog.vue'

interface Vm {
    createForm: { name: string; description?: string; expires_in_days?: number }
    createFormValid: boolean
    formRef: unknown
    createApiKey: () => void
}

describe('ApiKeyCreateDialog', () => {
    let wrapper: VueWrapper | undefined

    // `teleport: false` is essential here, not incidental. The global stub in
    // test-setup.ts keeps v-dialog's content out of the DOM, so <v-form>
    // never renders and its template ref never binds — which is precisely the
    // mechanism of the bug under test. Stubbed, these tests pass against the
    // broken code.
    // Vuetify's rules resolve across a microtask and a tick; asserting
    // immediately after setValue reads the previous state.
    const flushValidation = async (w: VueWrapper) => {
        await Promise.resolve()
        await w.vm.$nextTick()
        await Promise.resolve()
        await w.vm.$nextTick()
    }

    const body = () => new DOMWrapper(document.body)

    // Types a value the way a user does and lets Vuetify's rules settle.
    const typeName = async (w: VueWrapper, value: string) => {
        const input = body().find('input')
        await input.setValue(value)
        await input.trigger('blur')
        await flushValidation(w)
    }

    const mountDialog = (modelValue = true) =>
        mount(ApiKeyCreateDialog, {
            props: { modelValue, loading: false },
            global: { stubs: { teleport: false } },
        })

    afterEach(() => {
        wrapper?.unmount()
    })

    beforeEach(() => {
        vi.clearAllMocks()
    })

    // The regression this guards. `ref="createForm"` on <v-form> and
    // `const createForm = ref({...})` shared a name, and in <script setup> a
    // template ref binds by variable name — so Vue replaced the form data
    // with the VForm instance on mount. Typing a name and moving focus then
    // discarded it.
    it('keeps the form data separate from the VForm instance', () => {
        wrapper = mountDialog()
        const vm = wrapper.vm as unknown as Vm

        expect(vm.createForm).toEqual({
            name: '',
            description: '',
            expires_in_days: undefined,
        })
        // A component instance would not have these keys.
        expect('name' in vm.createForm).toBe(true)
    })

    it('retains a typed name across a re-render', async () => {
        wrapper = mountDialog()
        const vm = wrapper.vm as unknown as Vm

        vm.createForm.name = 'my test'
        await wrapper.vm.$nextTick()

        // Touching another field used to wipe the first one.
        vm.createForm.description = 'new field'
        await wrapper.vm.$nextTick()

        expect(vm.createForm.name).toBe('my test')
        expect(vm.createForm.description).toBe('new field')
    })

    it('emits the typed values on create', async () => {
        wrapper = mountDialog()
        const vm = wrapper.vm as unknown as Vm

        await typeName(wrapper, 'my test')
        vm.createForm.description = 'new field'
        await wrapper.vm.$nextTick()

        vm.createApiKey()

        const emitted = wrapper.emitted('create')
        expect(emitted).toBeTruthy()
        expect(emitted?.[0]?.[0]).toMatchObject({
            name: 'my test',
            description: 'new field',
        })
    })

    it('refuses to emit while the name is empty', () => {
        wrapper = mountDialog()
        const vm = wrapper.vm as unknown as Vm
        vm.createApiKey()

        expect(wrapper.emitted('create')).toBeFalsy()
    })

    // Validation had two owners: the field rule (`!!v`, which accepts '   ')
    // and a validateForm() that rejected it via trim(). Both wrote
    // createFormValid, so whichever ran last won. Driving the real input and
    // waiting for Vuetify is the only way to see which.
    it('rejects a whitespace-only name through the form, not just a helper', async () => {
        wrapper = mountDialog()
        const vm = wrapper.vm as unknown as Vm

        await typeName(wrapper, '   ')

        expect(vm.createFormValid).toBe(false)
    })

    it('refuses to submit a whitespace-only name', async () => {
        wrapper = mountDialog()
        const vm = wrapper.vm as unknown as Vm

        await typeName(wrapper, '   ')

        vm.createApiKey()

        expect(wrapper.emitted('create')).toBeFalsy()
    })

    it('accepts a real name typed into the field', async () => {
        wrapper = mountDialog()
        const vm = wrapper.vm as unknown as Vm

        await typeName(wrapper, 'my test')

        expect(vm.createFormValid).toBe(true)
    })

    it('clears the form when the dialog is reopened', async () => {
        wrapper = mountDialog(false)
        const vm = wrapper.vm as unknown as Vm

        await wrapper.setProps({ modelValue: true })
        vm.createForm.name = 'leftover'

        await wrapper.setProps({ modelValue: false })
        await wrapper.setProps({ modelValue: true })

        expect(vm.createForm.name).toBe('')
    })

    it('closes without emitting create', async () => {
        wrapper = mountDialog()
        const vm = wrapper.vm as unknown as Vm
        vm.createForm.name = 'abandoned'

        ;(wrapper.vm as unknown as { closeDialog: () => void }).closeDialog()
        await wrapper.vm.$nextTick()

        expect(wrapper.emitted('update:modelValue')?.[0]).toEqual([false])
        expect(wrapper.emitted('create')).toBeFalsy()
    })
})
