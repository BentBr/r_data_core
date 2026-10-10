import { describe, it, expect, vi, beforeEach, type Mock } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import WorkflowFormFields from './WorkflowFormFields.vue'
import type { DslStep } from './dsl/dsl-utils'

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        previewCron: vi.fn(),
    },
}))

vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({ t: (k: string) => k.split('.').pop() }),
}))

import { typedHttpClient } from '@/api/typed-client'

type WorkflowForm = {
    name: string
    description: string
    kind: 'consumer' | 'provider'
    enabled: boolean
    schedule_cron: string | null
    versioning_disabled: boolean
}

// Array.prototype.at() isn't available under the project's configured lib
// target, so index from the end manually instead.
const last = <T>(arr: T[]): T | undefined => arr[arr.length - 1]

const baseForm = (): WorkflowForm => ({
    name: 'My Workflow',
    description: 'desc',
    kind: 'consumer',
    enabled: true,
    schedule_cron: '*/5 * * * *',
    versioning_disabled: false,
})

const apiSourceStep = {
    from: {
        type: 'format',
        source: { source_type: 'api', config: {} },
        format: { format_type: 'json', options: {} },
        mapping: {},
    },
    transform: { type: 'none' },
    to: {
        type: 'format',
        output: { mode: 'db' },
        format: { format_type: 'json' },
        mapping: {},
    },
} as unknown as DslStep

const apiSourceWithEndpointStep = {
    from: {
        type: 'format',
        source: { source_type: 'api', config: { endpoint: '/webhook' } },
        format: { format_type: 'json', options: {} },
        mapping: {},
    },
    transform: { type: 'none' },
    to: {
        type: 'format',
        output: { mode: 'db' },
        format: { format_type: 'json' },
        mapping: {},
    },
} as unknown as DslStep

const apiOutputStringStep = {
    from: {
        type: 'format',
        source: { source_type: 'uri', config: { uri: 'http://x/y.csv' } },
        format: { format_type: 'csv', options: {} },
        mapping: {},
    },
    transform: { type: 'none' },
    to: {
        type: 'format',
        output: 'api',
        format: { format_type: 'json' },
        mapping: {},
    },
} as unknown as DslStep

const apiOutputModeStep = {
    from: {
        type: 'format',
        source: { source_type: 'uri', config: { uri: 'http://x/y.csv' } },
        format: { format_type: 'csv', options: {} },
        mapping: {},
    },
    transform: { type: 'none' },
    to: {
        type: 'format',
        output: { mode: 'api' },
        format: { format_type: 'json' },
        mapping: {},
    },
} as unknown as DslStep

const nonApiStep = {
    from: {
        type: 'format',
        source: { source_type: 'uri', config: { uri: 'http://x/y.csv' } },
        format: { format_type: 'csv', options: {} },
        mapping: {},
    },
    transform: { type: 'none' },
    to: {
        type: 'format',
        output: { mode: 'db' },
        format: { format_type: 'json' },
        mapping: {},
    },
} as unknown as DslStep

const nonFormatToStep = {
    from: {
        type: 'format',
        source: { source_type: 'uri', config: { uri: 'http://x/y.csv' } },
        format: { format_type: 'csv', options: {} },
        mapping: {},
    },
    transform: { type: 'none' },
    to: {
        type: 'entity',
        entity_definition: 'contact',
        mapping: {},
    },
} as unknown as DslStep

const mountFields = (overrides: Partial<Record<string, unknown>> = {}) =>
    mount(WorkflowFormFields, {
        props: {
            form: baseForm(),
            steps: [],
            cronError: null,
            cronHelp: '',
            nextRuns: [],
            ...overrides,
        },
    })

describe('WorkflowFormFields', () => {
    beforeEach(() => {
        vi.clearAllMocks()
    })

    it('renders form fields bound to the form prop', () => {
        const wrapper = mountFields()

        expect(wrapper.find('input[type="text"]').exists()).toBe(true)
        expect(wrapper.text()).not.toContain('undefined')
    })

    it('updates name, description, kind, enabled and versioning_disabled via real input', async () => {
        // The `form` prop is mutated in place via v-model on its nested
        // properties (no update:form is emitted for these fields), so assert
        // directly against the prop object rather than emitted events.
        const form = baseForm()
        const wrapper = mountFields({ form })

        await wrapper.find('input[type="text"]').setValue('Renamed Workflow')
        await wrapper.find('textarea').setValue('Updated description')

        const switches = wrapper.findAll('input[type="checkbox"]')
        await switches[0]?.setValue(false)
        await switches[1]?.setValue(true)

        const kindSelect = wrapper.findComponent({ name: 'VSelect' })
        await kindSelect.vm.$emit('update:modelValue', 'provider')
        await nextTick()

        expect(form.name).toBe('Renamed Workflow')
        expect(form.description).toBe('Updated description')
        expect(form.enabled).toBe(false)
        expect(form.versioning_disabled).toBe(true)
        expect(form.kind).toBe('provider')
    })

    it('does not treat a non-format step as having an api output', () => {
        const wrapper = mountFields({ steps: [nonFormatToStep] })

        const cronField = last(wrapper.findAll('input[type="text"]'))
        expect(cronField?.attributes('disabled')).toBeUndefined()
    })

    it('does not disable the cron field and shows no hint for a non-api workflow', () => {
        const wrapper = mountFields({ steps: [nonApiStep] })

        const cronField = last(wrapper.findAll('input[type="text"]'))
        expect(cronField?.attributes('disabled')).toBeUndefined()
    })

    it('disables the cron field when a step has an api source without an endpoint', () => {
        const wrapper = mountFields({ steps: [apiSourceStep] })

        const cronField = last(wrapper.findAll('input[type="text"]'))
        expect(cronField?.attributes('disabled')).toBeDefined()
        expect(wrapper.text()).toContain('cron_disabled_for_api_source')
    })

    it('does not treat an api source with an endpoint as a no-cron source', () => {
        const wrapper = mountFields({ steps: [apiSourceWithEndpointStep] })

        const cronField = last(wrapper.findAll('input[type="text"]'))
        expect(cronField?.attributes('disabled')).toBeUndefined()
    })

    it('disables the cron field when a step outputs via a string "api" mode', () => {
        const wrapper = mountFields({ steps: [apiOutputStringStep] })

        const cronField = last(wrapper.findAll('input[type="text"]'))
        expect(cronField?.attributes('disabled')).toBeDefined()
        expect(wrapper.text()).toContain('cron_disabled_for_api_output')
    })

    it('disables the cron field when a step outputs via an object mode "api"', () => {
        const wrapper = mountFields({ steps: [apiOutputModeStep] })

        const cronField = last(wrapper.findAll('input[type="text"]'))
        expect(cronField?.attributes('disabled')).toBeDefined()
        expect(wrapper.text()).toContain('cron_disabled_for_api_output')
    })

    it('treats an empty steps array as not having an api output', () => {
        const wrapper = mountFields({ steps: [] })

        const cronField = last(wrapper.findAll('input[type="text"]'))
        expect(cronField?.attributes('disabled')).toBeUndefined()
    })

    it('shows cronHelp text only when not using an api source or output', async () => {
        const wrapper = mountFields({ cronHelp: 'Runs every 5 minutes', steps: [] })
        expect(wrapper.text()).toContain('Runs every 5 minutes')

        await wrapper.setProps({ steps: [apiSourceStep] })
        expect(wrapper.text()).not.toContain('Runs every 5 minutes')
    })

    it('shows the next-runs preview joined by commas only when not using api source/output', async () => {
        const wrapper = mountFields({ nextRuns: ['2025-01-01T00:00', '2025-01-01T00:05'] })
        expect(wrapper.text()).toContain('2025-01-01T00:00, 2025-01-01T00:05')

        await wrapper.setProps({ steps: [apiOutputModeStep] })
        expect(wrapper.text()).not.toContain('2025-01-01T00:00, 2025-01-01T00:05')
    })

    it('clears the cron, error and next runs when steps switch to an api source', async () => {
        const wrapper = mountFields({
            steps: [],
            cronError: 'bad cron',
            nextRuns: ['2025-01-01T00:00'],
        })

        await wrapper.setProps({ steps: [apiSourceStep] })
        await nextTick()

        expect(wrapper.emitted('update:cronError')).toBeTruthy()
        expect(wrapper.emitted('update:cronError')?.[0]).toEqual([null])
        expect(wrapper.emitted('update:form')).toBeTruthy()
        const formEvent = wrapper.emitted('update:form')?.[0]?.[0] as WorkflowForm
        expect(formEvent.schedule_cron).toBeNull()
        expect(wrapper.emitted('update:nextRuns')?.[0]).toEqual([[]])
    })

    it('does not clear cron state when switching between two non-api step sets', async () => {
        const wrapper = mountFields({ steps: [nonApiStep] })

        await wrapper.setProps({ steps: [nonApiStep, nonApiStep] })
        await nextTick()

        expect(wrapper.emitted('update:form')).toBeFalsy()
    })

    describe('onCronChange', () => {
        it('emits cronChange and skips preview when an api source is active', async () => {
            const wrapper = mountFields({ steps: [apiSourceStep] })

            // The cron field is rendered disabled while an api source is
            // active, so simulate the v-text-field's own update event
            // directly rather than trying to type into a disabled control.
            const cronFieldComponent = last(wrapper.findAllComponents({ name: 'VTextField' }))
            cronFieldComponent?.vm.$emit('update:modelValue', '*/10 * * * *')
            await nextTick()

            expect(wrapper.emitted('cronChange')?.[0]).toEqual(['*/10 * * * *'])
            const cronErrorEvents = wrapper.emitted('update:cronError')
            expect(last(cronErrorEvents ?? [])).toEqual([null])
            const nextRunsEvents = wrapper.emitted('update:nextRuns')
            expect(last(nextRunsEvents ?? [])).toEqual([[]])
            expect(typedHttpClient.previewCron).not.toHaveBeenCalled()
        })

        it('clears next runs without calling the API when the cron value is blank', async () => {
            const wrapper = mountFields({ steps: [] })

            const cronField = last(wrapper.findAll('input[type="text"]'))
            await cronField?.setValue('   ')
            await nextTick()

            const nextRunsEvents = wrapper.emitted('update:nextRuns')
            expect(last(nextRunsEvents ?? [])).toEqual([[]])
            expect(typedHttpClient.previewCron).not.toHaveBeenCalled()
        })

        it('debounces and fetches a cron preview, emitting the resolved runs', async () => {
            ;(typedHttpClient.previewCron as Mock).mockResolvedValue([
                '2025-02-01T00:00:00Z',
                '2025-02-01T00:05:00Z',
            ])
            const wrapper = mountFields({ steps: [] })

            // Use a value distinct from the initial form.schedule_cron so the
            // v-text-field actually emits update:modelValue (Vuetify skips
            // the emit when setValue is a no-op against the current value).
            const cronField = last(wrapper.findAll('input[type="text"]'))
            await cronField?.setValue('0 0 * * *')
            await nextTick()

            expect(wrapper.emitted('cronChange')?.[0]).toEqual(['0 0 * * *'])
            const cronErrorEvents = wrapper.emitted('update:cronError')
            expect(last(cronErrorEvents ?? [])).toEqual([null])

            // Wait past the 350ms debounce window for the preview to resolve
            await new Promise(resolve => setTimeout(resolve, 450))

            expect(typedHttpClient.previewCron).toHaveBeenCalledWith('0 0 * * *')
            const nextRunsEvents = wrapper.emitted('update:nextRuns')
            expect(last(nextRunsEvents ?? [])).toEqual([
                ['2025-02-01T00:00:00Z', '2025-02-01T00:05:00Z'],
            ])
        })

        it('emits an empty next-runs list when the cron preview request fails', async () => {
            ;(typedHttpClient.previewCron as Mock).mockRejectedValue(new Error('invalid cron'))
            const wrapper = mountFields({ steps: [] })

            const cronField = last(wrapper.findAll('input[type="text"]'))
            await cronField?.setValue('garbage cron')
            await nextTick()

            await new Promise(resolve => setTimeout(resolve, 450))

            const nextRunsEvents = wrapper.emitted('update:nextRuns')
            expect(last(nextRunsEvents ?? [])).toEqual([[]])
        })

        it('debounces rapid successive cron edits into a single preview request', async () => {
            ;(typedHttpClient.previewCron as Mock).mockResolvedValue(['2025-03-01T00:00:00Z'])
            const wrapper = mountFields({ steps: [] })

            const cronField = last(wrapper.findAll('input[type="text"]'))
            await cronField?.setValue('* * * * *')
            await nextTick()
            await cronField?.setValue('*/2 * * * *')
            await nextTick()

            await new Promise(resolve => setTimeout(resolve, 450))

            expect(typedHttpClient.previewCron).toHaveBeenCalledTimes(1)
            expect(typedHttpClient.previewCron).toHaveBeenCalledWith('*/2 * * * *')
        })
    })
})
