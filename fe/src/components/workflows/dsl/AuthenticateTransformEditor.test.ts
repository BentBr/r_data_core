import { describe, it, expect, vi, beforeEach, type Mock } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import AuthenticateTransformEditor from './AuthenticateTransformEditor.vue'
import type { Transform } from './contracts'

const mockGetEntityFields: Mock = vi.fn()
const mockGetEntityDefinitions: Mock = vi.fn()

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getEntityFields: (entityType: string) => mockGetEntityFields(entityType),
    },
}))

vi.mock('@/composables/useEntityDefinitions', () => ({
    useEntityDefinitions: () => ({
        entityDefinitions: { value: mockGetEntityDefinitions() },
        loadEntityDefinitions: vi.fn().mockResolvedValue(undefined),
    }),
}))

vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({ t: (k: string) => k }),
}))

function makeTransform(overrides: Partial<Extract<Transform, { type: 'authenticate' }>> = {}) {
    return {
        type: 'authenticate',
        entity_type: '',
        identifier_field: '',
        password_field: '',
        input_identifier: '',
        input_password: '',
        target_token: '',
        ...overrides,
    } as Transform
}

describe('AuthenticateTransformEditor', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        mockGetEntityFields.mockResolvedValue([
            { name: 'uuid', type: 'Uuid', required: true, system: true },
            { name: 'email', type: 'string', required: true, system: false },
            { name: 'password_hash', type: 'Password', required: true, system: false },
            { name: 'created_at', type: 'DateTime', required: true, system: true },
        ])
        mockGetEntityDefinitions.mockReturnValue([
            { entity_type: 'user', display_name: 'User' },
            { entity_type: 'customer', display_name: 'Customer' },
        ])
    })

    it('renders all core fields for an empty authenticate transform', async () => {
        const wrapper = mount(AuthenticateTransformEditor, {
            props: { modelValue: makeTransform() },
        })
        await nextTick()

        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        // entity type select, identifier field select, password field select
        expect(selects.length).toBeGreaterThanOrEqual(3)

        const textFields = wrapper.findAllComponents({ name: 'VTextField' })
        // input_identifier, input_password, target_token, token_expiry
        expect(textFields.length).toBeGreaterThanOrEqual(4)
    })

    it('loads entity fields when entity_type is set initially', async () => {
        const wrapper = mount(AuthenticateTransformEditor, {
            props: { modelValue: makeTransform({ entity_type: 'user' }) },
        })
        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 50))

        expect(mockGetEntityFields).toHaveBeenCalledWith('user')

        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        const identifierSelect = selects.find(s => !s.props('disabled'))
        expect(identifierSelect).toBeTruthy()
    })

    it('restricts password field dropdown to Password-type fields when available', async () => {
        const wrapper = mount(AuthenticateTransformEditor, {
            props: { modelValue: makeTransform({ entity_type: 'user' }) },
        })
        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 50))

        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        const passwordSelect = selects.find(s => {
            const items = s.props('items') as string[] | undefined
            return Array.isArray(items) && items.length === 1 && items[0] === 'password_hash'
        })
        expect(passwordSelect).toBeTruthy()
    })

    it('falls back to all entity fields for password dropdown when no Password-type field exists', async () => {
        mockGetEntityFields.mockResolvedValueOnce([
            { name: 'uuid', type: 'Uuid', required: true, system: true },
            { name: 'email', type: 'string', required: true, system: false },
        ])
        const wrapper = mount(AuthenticateTransformEditor, {
            props: { modelValue: makeTransform({ entity_type: 'user' }) },
        })
        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 50))

        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        // index 2 is the password field select (0: entity type, 1: identifier field)
        const passwordSelect = selects[2]
        expect(passwordSelect.props('items')).toEqual(['email'])
    })

    it('emits updated entity_type and triggers field reload on change', async () => {
        const wrapper = mount(AuthenticateTransformEditor, {
            props: { modelValue: makeTransform() },
        })
        await nextTick()

        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        const entityTypeSelect = selects[0]
        mockGetEntityFields.mockClear()
        await entityTypeSelect.vm.$emit('update:modelValue', 'customer')
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue')
        expect(emitted).toBeTruthy()
        const updated = emitted![emitted!.length - 1][0] as Transform
        expect(updated.type).toBe('authenticate')
        if (updated.type === 'authenticate') {
            expect(updated.entity_type).toBe('customer')
        }
        expect(mockGetEntityFields).toHaveBeenCalledWith('customer')
    })

    it('updates identifier_field when identifier select changes', async () => {
        const wrapper = mount(AuthenticateTransformEditor, {
            props: { modelValue: makeTransform({ entity_type: 'user' }) },
        })
        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 50))

        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        const identifierSelect = selects[1]
        await identifierSelect.vm.$emit('update:modelValue', 'email')
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue')
        const updated = emitted![emitted!.length - 1][0] as Transform
        if (updated.type === 'authenticate') {
            expect(updated.identifier_field).toBe('email')
        }
    })

    it('updates password_field when password select changes', async () => {
        const wrapper = mount(AuthenticateTransformEditor, {
            props: { modelValue: makeTransform({ entity_type: 'user' }) },
        })
        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 50))

        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        const passwordSelect = selects[2]
        await passwordSelect.vm.$emit('update:modelValue', 'password_hash')
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue')
        const updated = emitted![emitted!.length - 1][0] as Transform
        if (updated.type === 'authenticate') {
            expect(updated.password_field).toBe('password_hash')
        }
    })

    it('updates input_identifier, input_password, and target_token text fields', async () => {
        const wrapper = mount(AuthenticateTransformEditor, {
            props: { modelValue: makeTransform() },
        })
        await nextTick()

        const textFields = wrapper.findAllComponents({ name: 'VTextField' })
        const identifierField = textFields.find(
            tf => (tf.props('label') as string) === 'workflows.dsl.auth_input_identifier'
        )
        const passwordField = textFields.find(
            tf => (tf.props('label') as string) === 'workflows.dsl.auth_input_password'
        )
        const targetTokenField = textFields.find(
            tf => (tf.props('label') as string) === 'workflows.dsl.auth_target_token'
        )
        expect(identifierField).toBeTruthy()
        expect(passwordField).toBeTruthy()
        expect(targetTokenField).toBeTruthy()

        await identifierField!.vm.$emit('update:modelValue', '{{email}}')
        await nextTick()
        let emitted = wrapper.emitted('update:modelValue')
        let updated = emitted![emitted!.length - 1][0] as Transform
        if (updated.type === 'authenticate') {
            expect(updated.input_identifier).toBe('{{email}}')
        }

        await passwordField!.vm.$emit('update:modelValue', '{{password}}')
        await nextTick()
        emitted = wrapper.emitted('update:modelValue')
        updated = emitted![emitted!.length - 1][0] as Transform
        if (updated.type === 'authenticate') {
            expect(updated.input_password).toBe('{{password}}')
        }

        await targetTokenField!.vm.$emit('update:modelValue', 'auth_token')
        await nextTick()
        emitted = wrapper.emitted('update:modelValue')
        updated = emitted![emitted!.length - 1][0] as Transform
        if (updated.type === 'authenticate') {
            expect(updated.target_token).toBe('auth_token')
        }
    })

    it('sets token_expiry_seconds to a positive number', async () => {
        const wrapper = mount(AuthenticateTransformEditor, {
            props: { modelValue: makeTransform() },
        })
        await nextTick()

        const textFields = wrapper.findAllComponents({ name: 'VTextField' })
        const expiryField = textFields.find(
            tf => (tf.props('label') as string) === 'workflows.dsl.auth_token_expiry'
        )
        expect(expiryField).toBeTruthy()

        await expiryField!.vm.$emit('update:modelValue', '3600')
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue')
        const updated = emitted![emitted!.length - 1][0] as Transform
        if (updated.type === 'authenticate') {
            expect(updated.token_expiry_seconds).toBe(3600)
        }
    })

    it('clears token_expiry_seconds when given an empty or non-positive value', async () => {
        const wrapper = mount(AuthenticateTransformEditor, {
            props: { modelValue: makeTransform({ token_expiry_seconds: 60 }) },
        })
        await nextTick()

        const textFields = wrapper.findAllComponents({ name: 'VTextField' })
        const expiryField = textFields.find(
            tf => (tf.props('label') as string) === 'workflows.dsl.auth_token_expiry'
        )

        await expiryField!.vm.$emit('update:modelValue', '0')
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue')
        const updated = emitted![emitted!.length - 1][0] as Transform
        if (updated.type === 'authenticate') {
            expect(updated.token_expiry_seconds).toBeUndefined()
        }
    })

    it('renders existing extra_claims as pairs in the mapping table', async () => {
        const wrapper = mount(AuthenticateTransformEditor, {
            props: {
                modelValue: makeTransform({ extra_claims: { role: 'user_role' } }),
            },
        })
        await nextTick()

        const rows = wrapper.findAll('tbody tr')
        expect(rows.length).toBe(1)
    })

    it('adds a new extra claim pair when add button clicked', async () => {
        const wrapper = mount(AuthenticateTransformEditor, {
            props: { modelValue: makeTransform() },
        })
        await nextTick()

        const addButton = wrapper.findAll('button').find(b => b.text().includes('add_mapping'))
        expect(addButton).toBeTruthy()

        await addButton!.trigger('click')
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue')
        expect(emitted).toBeTruthy()
        const updated = emitted![emitted!.length - 1][0] as Transform
        if (updated.type === 'authenticate') {
            expect(Object.keys(updated.extra_claims ?? {}).length).toBe(1)
        }
    })

    it('updates an extra claim pair via the mapping table', async () => {
        const wrapper = mount(AuthenticateTransformEditor, {
            props: {
                modelValue: makeTransform({ extra_claims: { role: 'user_role' } }),
            },
        })
        await nextTick()

        const mappingTable = wrapper.findComponent({ name: 'MappingTable' })
        expect(mappingTable.exists()).toBe(true)

        await mappingTable.vm.$emit('update-pair', 0, { k: 'scope', v: 'scope_field' })
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue')
        const updated = emitted![emitted!.length - 1][0] as Transform
        if (updated.type === 'authenticate') {
            expect(updated.extra_claims).toEqual({ scope: 'scope_field' })
        }
    })

    it('deletes an extra claim pair via the mapping table', async () => {
        const wrapper = mount(AuthenticateTransformEditor, {
            props: {
                modelValue: makeTransform({
                    extra_claims: { role: 'user_role', scope: 'scope_field' },
                }),
            },
        })
        await nextTick()

        const mappingTable = wrapper.findComponent({ name: 'MappingTable' })
        await mappingTable.vm.$emit('delete-pair', 0)
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue')
        const updated = emitted![emitted!.length - 1][0] as Transform
        if (updated.type === 'authenticate') {
            expect(updated.extra_claims).toEqual({ scope: 'scope_field' })
        }
    })

    it('clears entity and password fields when getEntityFields rejects', async () => {
        mockGetEntityFields.mockRejectedValueOnce(new Error('network error'))
        const wrapper = mount(AuthenticateTransformEditor, {
            props: { modelValue: makeTransform({ entity_type: 'user' }) },
        })
        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 50))

        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        const identifierSelect = selects[1]
        expect(identifierSelect.props('disabled')).toBe(true)
    })

    it('shows the no-password-type hint when entity fields loaded without a Password field', async () => {
        mockGetEntityFields.mockResolvedValueOnce([
            { name: 'email', type: 'string', required: true, system: false },
        ])
        const wrapper = mount(AuthenticateTransformEditor, {
            props: { modelValue: makeTransform({ entity_type: 'user' }) },
        })
        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 50))

        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        const passwordSelect = selects[2]
        expect(passwordSelect.props('hint')).toBe(
            'workflows.dsl.auth_password_field_no_password_type'
        )
    })
})
