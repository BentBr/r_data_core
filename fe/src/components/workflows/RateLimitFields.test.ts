import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import RateLimitFields from './RateLimitFields.vue'
import { DEFAULT_RATE_LIMIT } from '@/types/schemas/workflow'
import type { WorkflowRateLimit } from '@/types/schemas/workflow'

const mountFields = (modelValue: WorkflowRateLimit = { ...DEFAULT_RATE_LIMIT }) =>
    mount(RateLimitFields, { props: { modelValue } })

const lastEmit = (wrapper: ReturnType<typeof mountFields>): WorkflowRateLimit => {
    const emitted = wrapper.emitted('update:modelValue') as Array<[WorkflowRateLimit]> | undefined
    if (!emitted?.length) {
        throw new Error('component emitted no update')
    }
    return emitted[emitted.length - 1][0]
}

/** Run a v-text-field's own rules and return the messages it produced. */
const validateField = async (
    wrapper: ReturnType<typeof mountFields>,
    testid: string
): Promise<string[]> => {
    const field = wrapper.findComponent(`[data-testid="${testid}"]`) as unknown as {
        vm: { validate: () => Promise<string[]> }
    }
    return field.vm.validate()
}

describe('RateLimitFields', () => {
    it('hides the inputs until the switch is on', async () => {
        const wrapper = mountFields()

        expect(wrapper.find('[data-testid="rate-limit-max"]').exists()).toBe(false)
        expect(wrapper.find('[data-testid="rate-limit-window"]').exists()).toBe(false)

        await wrapper.setProps({ modelValue: { ...DEFAULT_RATE_LIMIT, enabled: true } })
        await nextTick()

        expect(wrapper.find('[data-testid="rate-limit-max"]').exists()).toBe(true)
        expect(wrapper.find('[data-testid="rate-limit-window"]').exists()).toBe(true)
    })

    it('emits the whole limit when the switch is flipped', async () => {
        const wrapper = mountFields()

        await wrapper.find('[data-testid="rate-limit-enabled"] input').setValue(true)

        expect(lastEmit(wrapper)).toEqual({ enabled: true, max_requests: 10, window_minutes: 60 })
    })

    it('emits numeric values, not the strings the inputs hand back', async () => {
        const wrapper = mountFields({ enabled: true, max_requests: 10, window_minutes: 60 })

        await wrapper.find('[data-testid="rate-limit-max"] input').setValue('25')
        expect(lastEmit(wrapper).max_requests).toBe(25)

        await wrapper.find('[data-testid="rate-limit-window"] input').setValue('5')
        expect(lastEmit(wrapper).window_minutes).toBe(5)
    })

    it('shows the values it is given', () => {
        const wrapper = mountFields({ enabled: true, max_requests: 5, window_minutes: 15 })

        const max = wrapper.find('[data-testid="rate-limit-max"] input').element as HTMLInputElement
        const window = wrapper.find('[data-testid="rate-limit-window"] input')
            .element as HTMLInputElement

        expect(max.value).toBe('5')
        expect(window.value).toBe('15')
    })

    // An HTML `min` attribute does not stop a save. The backend reads a zero or
    // a fraction as no limit at all, so without these rules the form could be
    // saved showing "enabled" while nothing is enforced.
    //
    // The fields are controlled by the prop, so the value under test has to
    // arrive as a prop - typing into the DOM alone would be reverted.
    it.each([
        [0, 'zero'],
        [2.5, 'a fraction'],
        [-1, 'a negative number'],
        [Number.NaN, 'a cleared field'],
    ])('rejects %s (%s)', async value => {
        for (const testid of ['rate-limit-max', 'rate-limit-window']) {
            const key = testid === 'rate-limit-max' ? 'max_requests' : 'window_minutes'
            const wrapper = mountFields({
                enabled: true,
                max_requests: 10,
                window_minutes: 60,
                [key]: value,
            } as WorkflowRateLimit)

            const errors = await validateField(wrapper, testid)

            expect(errors.length, `${testid} must reject ${value}`).toBeGreaterThan(0)
        }
    })

    it('accepts a whole number of at least one', async () => {
        const wrapper = mountFields({ enabled: true, max_requests: 1, window_minutes: 1 })

        for (const testid of ['rate-limit-max', 'rate-limit-window']) {
            const errors = await validateField(wrapper, testid)
            expect(errors, `${testid} must accept 1`).toEqual([])
        }
    })
})
