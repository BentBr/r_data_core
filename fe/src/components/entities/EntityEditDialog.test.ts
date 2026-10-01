import { mount, VueWrapper } from '@vue/test-utils'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import EntityEditDialog from './EntityEditDialog.vue'
import type { DynamicEntity, EntityDefinition } from '@/types/schemas'

// Mock Translations
vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({
        t: (key: string) => key,
    }),
}))

vi.mock('@/components/common/SmartIcon.vue', () => ({
    default: { template: '<div class="smart-icon"></div>' },
}))

vi.mock('@/design-system/components', () => ({
    getDialogMaxWidth: () => '600px',
    buttonConfigs: {
        text: { variant: 'text' },
        primary: { color: 'primary', variant: 'flat' },
    },
}))

const makeDefinition = (overrides: Partial<EntityDefinition> = {}): EntityDefinition =>
    ({
        uuid: 'def-uuid-1',
        entity_type: 'test_type',
        display_name: 'Test Type',
        published: true,
        allow_children: true,
        fields: [{ name: 'name', display_name: 'Name', field_type: 'String', required: true }],
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        created_by: 'user-uuid',
        version: 1,
        ...overrides,
    }) as unknown as EntityDefinition

const makeEntity = (overrides: Partial<DynamicEntity> = {}): DynamicEntity =>
    ({
        entity_type: 'test_type',
        field_data: {
            uuid: 'entity-uuid-1',
            name: 'Ada',
            published: true,
        },
        ...overrides,
    }) as unknown as DynamicEntity

describe('EntityEditDialog', () => {
    let wrapper: VueWrapper

    beforeEach(() => {
        vi.clearAllMocks()
    })

    afterEach(() => {
        wrapper.unmount()
    })

    const mountComponent = (props: Record<string, unknown> = {}) =>
        mount(EntityEditDialog, {
            props: {
                modelValue: false,
                entity: makeEntity(),
                entityDefinition: makeDefinition(),
                ...props,
            },
        })

    // v-dialog content is teleported; the global test harness stubs
    // teleport (so form refs don't bind). Disabling the stub here lets the
    // real v-form mount so `form.value.validate()` actually resolves.
    const mountOpenWithRealForm = async (props: Record<string, unknown> = {}) => {
        const w = mount(EntityEditDialog, {
            global: { stubs: { teleport: false } },
            props: {
                modelValue: false,
                entity: makeEntity(),
                entityDefinition: makeDefinition(),
                ...props,
            },
        })
        await w.setProps({ modelValue: true })
        await w.vm.$nextTick()
        return w
    }

    describe('dialog visibility', () => {
        it('emits update:modelValue when dialog closes', async () => {
            wrapper = mountComponent({ modelValue: true })
            const vm = wrapper.vm as unknown as { closeDialog: () => void }
            vm.closeDialog()
            await wrapper.vm.$nextTick()

            expect(wrapper.emitted('update:modelValue')).toBeTruthy()
            expect(wrapper.emitted('update:modelValue')?.[0]).toEqual([false])
        })

        it('resets formData when closed', async () => {
            wrapper = mountComponent({ modelValue: true })

            const vm = wrapper.vm as unknown as {
                closeDialog: () => void
                formData: { data: Record<string, unknown>; parent_uuid: string | null | undefined }
            }
            vm.closeDialog()
            await wrapper.vm.$nextTick()

            expect(vm.formData.data).toEqual({ published: false })
            expect(vm.formData.parent_uuid).toBeUndefined()
        })

        it('initializes form data from entity when dialog opens', async () => {
            wrapper = mountComponent({ modelValue: false })

            await wrapper.setProps({ modelValue: true })
            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as {
                formData: { data: Record<string, unknown>; parent_uuid: string | null }
            }

            expect(vm.formData.data.name).toBe('Ada')
            expect(vm.formData.data.published).toBe(true)
        })

        it('defaults published to false when absent on entity', async () => {
            wrapper = mountComponent({
                modelValue: false,
                entity: makeEntity({ field_data: { uuid: 'x', name: 'Bob' } }),
            })

            await wrapper.setProps({ modelValue: true })
            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as { formData: { data: Record<string, unknown> } }
            expect(vm.formData.data.published).toBe(false)
        })
    })

    describe('entity changes while open', () => {
        it('re-initializes form data when entity prop changes while dialog is open', async () => {
            wrapper = mountComponent({ modelValue: true })
            await wrapper.vm.$nextTick()

            await wrapper.setProps({
                entity: makeEntity({
                    field_data: { uuid: 'entity-uuid-2', name: 'Grace', published: false },
                }),
            })
            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as { formData: { data: Record<string, unknown> } }
            expect(vm.formData.data.name).toBe('Grace')
        })
    })

    describe('parentUuidValue', () => {
        it('reflects and updates parent_uuid', async () => {
            wrapper = mountComponent({ modelValue: true })
            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as {
                parentUuidValue: string | null
                formData: { parent_uuid: string | null }
            }

            vm.parentUuidValue = 'parent-123'
            expect(vm.formData.parent_uuid).toBe('parent-123')

            vm.parentUuidValue = null
            expect(vm.formData.parent_uuid).toBeNull()
        })
    })

    describe('availableParents', () => {
        it('returns empty array regardless of entity (not yet implemented)', async () => {
            wrapper = mountComponent({ modelValue: true })
            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as { availableParents: unknown[] }
            expect(vm.availableParents).toEqual([])
        })

        it('returns empty array when entity is null', async () => {
            wrapper = mountComponent({ modelValue: true, entity: null })
            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as { availableParents: unknown[] }
            expect(vm.availableParents).toEqual([])
        })
    })

    describe('updateEntity', () => {
        it('emits update with processed data on success', async () => {
            wrapper = await mountOpenWithRealForm()

            const vm = wrapper.vm as unknown as { updateEntity: () => Promise<void> }
            await vm.updateEntity()

            expect(wrapper.emitted('update')).toBeTruthy()
            const payload = wrapper.emitted('update')?.[0]?.[0] as {
                data: Record<string, unknown>
                parent_uuid: string | null
            }
            expect(payload.data.name).toBe('Ada')
        })

        it('sets field error and skips emit when JSON field value is invalid', async () => {
            wrapper = await mountOpenWithRealForm({
                entityDefinition: makeDefinition({
                    fields: [
                        {
                            name: 'meta',
                            display_name: 'Meta',
                            field_type: 'Json',
                        },
                    ] as unknown as EntityDefinition['fields'],
                }),
                entity: makeEntity({
                    field_data: { uuid: 'entity-uuid-1', meta: '{not valid json' },
                }),
            })

            const vm = wrapper.vm as unknown as {
                updateEntity: () => Promise<void>
                getFieldErrorMessages: (name: string) => string[]
            }
            await vm.updateEntity()

            expect(wrapper.emitted('update')).toBeFalsy()
            expect(vm.getFieldErrorMessages('meta').length).toBeGreaterThan(0)
        })

        it('clears field errors once the user edits the form again', async () => {
            wrapper = await mountOpenWithRealForm({
                entityDefinition: makeDefinition({
                    fields: [
                        {
                            name: 'meta',
                            display_name: 'Meta',
                            field_type: 'Json',
                        },
                    ] as unknown as EntityDefinition['fields'],
                }),
                entity: makeEntity({
                    field_data: { uuid: 'entity-uuid-1', meta: '{not valid json' },
                }),
            })

            const vm = wrapper.vm as unknown as {
                updateEntity: () => Promise<void>
                getFieldErrorMessages: (name: string) => string[]
                formData: { data: Record<string, unknown> }
            }
            await vm.updateEntity()
            expect(vm.getFieldErrorMessages('meta').length).toBeGreaterThan(0)

            vm.formData.data.meta = 'anything else'
            await wrapper.vm.$nextTick()

            expect(vm.getFieldErrorMessages('meta')).toEqual([])
        })

        it('never blocks submission on an invalid form (missing await on validate())', async () => {
            // Production bug: `if (!form.value?.validate())` never awaits the
            // Promise VForm.validate() returns, so the Promise object (always
            // truthy) defeats the negation and the guard never short-circuits.
            // A required field left empty should block submission but does not.
            wrapper = await mountOpenWithRealForm({
                entityDefinition: makeDefinition({
                    fields: [
                        {
                            name: 'name',
                            display_name: 'Name',
                            field_type: 'String',
                            required: true,
                        },
                    ] as unknown as EntityDefinition['fields'],
                }),
                entity: makeEntity({ field_data: { uuid: 'entity-uuid-1', name: '' } }),
            })

            const vm = wrapper.vm as unknown as { updateEntity: () => Promise<void> }
            await vm.updateEntity()

            // Documents current (buggy) behaviour: emits despite the empty
            // required field, because the validate() guard is a no-op.
            expect(wrapper.emitted('update')).toBeTruthy()
        })
    })

    describe('setFieldErrors exposed method', () => {
        it('allows parent to set field errors via exposed method', async () => {
            wrapper = mountComponent({ modelValue: true })
            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as {
                setFieldErrors: (errors: Record<string, string>) => void
                getFieldErrorMessages: (name: string) => string[]
            }
            vm.setFieldErrors({ name: 'Name already taken' })

            expect(vm.getFieldErrorMessages('name')).toEqual(['Name already taken'])
        })
    })
})
