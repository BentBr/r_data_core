import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createWebHistory } from 'vue-router'
import EntityDefinitionsPage from './EntityDefinitionsPage.vue'
import type { EntityDefinition, FieldDefinition } from '@/types/schemas'

const mockGetEntityDefinitions = vi.fn()
const mockGetEntityDefinition = vi.fn()
const mockCreateEntityDefinition = vi.fn()
const mockUpdateEntityDefinition = vi.fn()
const mockDeleteEntityDefinition = vi.fn()
const mockListEntityDefinitionVersions = vi.fn()

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getEntityDefinitions: (page?: number, itemsPerPage?: number) =>
            mockGetEntityDefinitions(page, itemsPerPage),
        getEntityDefinition: (uuid: string) => mockGetEntityDefinition(uuid),
        createEntityDefinition: (data: Record<string, unknown>) => mockCreateEntityDefinition(data),
        updateEntityDefinition: (uuid: string, data: Record<string, unknown>) =>
            mockUpdateEntityDefinition(uuid, data),
        deleteEntityDefinition: (uuid: string) => mockDeleteEntityDefinition(uuid),
        listEntityDefinitionVersions: (uuid: string) => mockListEntityDefinitionVersions(uuid),
    },
    ValidationError: class ValidationError extends Error {
        violations: Array<{ field: string; message: string }>

        constructor(violations: Array<{ field: string; message: string }>) {
            super('validation')
            this.violations = violations
        }
    },
}))

vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({ t: (k: string) => k.split('.').pop() }),
}))

const showSuccess = vi.fn()
const showError = vi.fn()
vi.mock('@/composables/useSnackbar', () => ({
    useSnackbar: () => ({
        currentSnackbar: null,
        showSuccess,
        showError,
    }),
}))

const mockHasPermission = vi.fn()
vi.mock('@/stores/auth', () => ({
    useAuthStore: () => ({
        isAuthenticated: true,
        hasPermission: mockHasPermission,
    }),
}))

const router = createRouter({
    history: createWebHistory(),
    routes: [{ path: '/entity-definitions', component: EntityDefinitionsPage }],
})

const makeDefinition = (overrides: Partial<EntityDefinition> = {}): EntityDefinition =>
    ({
        uuid: 'def-uuid-1',
        entity_type: 'Customer',
        display_name: 'Customer',
        allow_children: true,
        published: true,
        fields: [],
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        created_by: 'user-uuid',
        version: 1,
        ...overrides,
    }) as unknown as EntityDefinition

describe('EntityDefinitionsPage', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        mockGetEntityDefinitions.mockResolvedValue({ data: [makeDefinition()] })
        mockGetEntityDefinition.mockResolvedValue(makeDefinition())
        mockCreateEntityDefinition.mockResolvedValue(makeDefinition())
        mockUpdateEntityDefinition.mockResolvedValue(makeDefinition())
        mockDeleteEntityDefinition.mockResolvedValue({ message: 'deleted' })
        mockListEntityDefinitionVersions.mockResolvedValue([])
        // Default: user has create permission
        mockHasPermission.mockImplementation((namespace: string, permission: string) => {
            return (
                namespace === 'EntityDefinitions' &&
                (permission === 'Create' || permission === 'Admin')
            )
        })
    })

    const mountReady = async () => {
        const wrapper = mount(EntityDefinitionsPage, { global: { plugins: [router] } })
        await vi.waitUntil(() => mockGetEntityDefinitions.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()
        return wrapper
    }

    it('shows create button when user has EntityDefinitions:Create permission', async () => {
        mockHasPermission.mockImplementation((namespace: string, permission: string) => {
            return namespace === 'EntityDefinitions' && permission === 'Create'
        })

        const wrapper = await mountReady()

        expect((wrapper.vm as any).canCreateEntityDefinition).toBe(true)
    })

    it('shows create button when user has EntityDefinitions:Admin permission', async () => {
        mockHasPermission.mockImplementation((namespace: string, permission: string) => {
            return namespace === 'EntityDefinitions' && permission === 'Admin'
        })

        const wrapper = await mountReady()

        expect((wrapper.vm as any).canCreateEntityDefinition).toBe(true)
    })

    it('hides create button when user lacks create permissions', async () => {
        mockHasPermission.mockImplementation(() => false)

        const wrapper = await mountReady()

        expect((wrapper.vm as any).canCreateEntityDefinition).toBe(false)
    })

    it('loadEntityDefinitions records an error message on failure', async () => {
        mockGetEntityDefinitions.mockReset()
        mockGetEntityDefinitions.mockRejectedValue(new Error('network down'))

        const wrapper = mount(EntityDefinitionsPage, { global: { plugins: [router] } })
        await vi.waitUntil(() => mockGetEntityDefinitions.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        const vm = wrapper.vm as unknown as { error: string; loading: boolean }
        expect(vm.error).toBe('network down')
        expect(vm.loading).toBe(false)
    })

    describe('handleTreeSelection', () => {
        it('selects a definition by uuid', async () => {
            const wrapper = await mountReady()

            const vm = wrapper.vm as unknown as {
                handleTreeSelection: (items: string[]) => void
                selectedDefinition: EntityDefinition | null
            }
            vm.handleTreeSelection(['def-uuid-1'])
            await wrapper.vm.$nextTick()

            expect(vm.selectedDefinition?.uuid).toBe('def-uuid-1')
        })

        it('clears selection when no items are passed', async () => {
            const wrapper = await mountReady()

            const vm = wrapper.vm as unknown as {
                handleTreeSelection: (items: string[]) => void
                selectedDefinition: EntityDefinition | null
            }
            vm.handleTreeSelection(['def-uuid-1'])
            vm.handleTreeSelection([])

            expect(vm.selectedDefinition).toBeNull()
        })

        it('toggles group expansion for group- prefixed ids', async () => {
            const wrapper = await mountReady()

            const vm = wrapper.vm as unknown as {
                handleTreeSelection: (items: string[]) => void
                expandedGroups: string[]
            }
            vm.handleTreeSelection(['group-Sales'])
            expect(vm.expandedGroups).toContain('group-Sales')

            vm.handleTreeSelection(['group-Sales'])
            expect(vm.expandedGroups).not.toContain('group-Sales')
        })
    })

    describe('handleItemClick', () => {
        it('toggles expansion for group tree nodes', async () => {
            const wrapper = await mountReady()

            const vm = wrapper.vm as unknown as {
                handleItemClick: (item: { entity_type: string; id: string }) => Promise<void>
                expandedGroups: string[]
            }
            await vm.handleItemClick({ entity_type: 'group', id: 'group-Sales' })
            expect(vm.expandedGroups).toContain('group-Sales')
        })

        it('loads and selects a definition from the server', async () => {
            const wrapper = await mountReady()
            mockGetEntityDefinition.mockResolvedValue(
                makeDefinition({ display_name: 'Customer (reloaded)' })
            )

            const vm = wrapper.vm as unknown as {
                handleItemClick: (item: { entity_type: string; uuid: string }) => Promise<void>
                selectedDefinition: EntityDefinition | null
            }
            await vm.handleItemClick({ entity_type: 'Customer', uuid: 'def-uuid-1' })

            expect(mockGetEntityDefinition).toHaveBeenCalledWith('def-uuid-1')
            expect(vm.selectedDefinition?.display_name).toBe('Customer (reloaded)')
        })

        it('reports errors via the error handler', async () => {
            const wrapper = await mountReady()
            mockGetEntityDefinition.mockRejectedValue(new Error('not found'))

            const vm = wrapper.vm as unknown as {
                handleItemClick: (item: { entity_type: string; uuid: string }) => Promise<void>
                loading: boolean
            }
            await vm.handleItemClick({ entity_type: 'Customer', uuid: 'def-uuid-1' })

            expect(vm.loading).toBe(false)
            expect(showError).toHaveBeenCalled()
        })
    })

    describe('hasUnsavedChanges', () => {
        it('is false when nothing is selected', async () => {
            const wrapper = await mountReady()
            const vm = wrapper.vm as unknown as { hasUnsavedChanges: boolean }
            expect(vm.hasUnsavedChanges).toBe(false)
        })

        it('is true when field count differs from the original', async () => {
            const wrapper = await mountReady()
            const vm = wrapper.vm as unknown as {
                handleTreeSelection: (items: string[]) => void
                selectedDefinition: EntityDefinition | null
                hasUnsavedChanges: boolean
            }
            vm.handleTreeSelection(['def-uuid-1'])
            await wrapper.vm.$nextTick()

            vm.selectedDefinition!.fields.push({
                name: 'new_field',
                display_name: 'New',
                field_type: 'String',
            } as unknown as FieldDefinition)
            await wrapper.vm.$nextTick()

            expect(vm.hasUnsavedChanges).toBe(true)
        })

        it('is true when a field property changes', async () => {
            const wrapper = await mountReady()
            mockGetEntityDefinitions.mockResolvedValue({
                data: [
                    makeDefinition({
                        fields: [
                            {
                                name: 'f1',
                                display_name: 'F1',
                                field_type: 'String',
                                required: false,
                            },
                        ] as unknown as FieldDefinition[],
                    }),
                ],
            })
            const wrapper2 = await mountReady()
            const vm = wrapper2.vm as unknown as {
                handleTreeSelection: (items: string[]) => void
                selectedDefinition: EntityDefinition | null
                hasUnsavedChanges: boolean
            }
            vm.handleTreeSelection(['def-uuid-1'])
            await wrapper2.vm.$nextTick()

            vm.selectedDefinition!.fields[0].required = true
            await wrapper2.vm.$nextTick()

            expect(vm.hasUnsavedChanges).toBe(true)
            wrapper.unmount()
        })
    })

    describe('saveChanges', () => {
        it('persists changes and reloads the definition', async () => {
            const wrapper = await mountReady()
            const vm = wrapper.vm as unknown as {
                handleTreeSelection: (items: string[]) => void
                saveChanges: () => Promise<void>
                savingChanges: boolean
            }
            vm.handleTreeSelection(['def-uuid-1'])
            await wrapper.vm.$nextTick()

            await vm.saveChanges()

            expect(mockUpdateEntityDefinition).toHaveBeenCalled()
            expect(mockGetEntityDefinition).toHaveBeenCalledWith('def-uuid-1')
            expect(vm.savingChanges).toBe(false)
            expect(showSuccess).toHaveBeenCalledWith('changes_saved')
        })

        it('does nothing when there is no selected definition', async () => {
            const wrapper = await mountReady()
            const vm = wrapper.vm as unknown as { saveChanges: () => Promise<void> }
            await vm.saveChanges()

            expect(mockUpdateEntityDefinition).not.toHaveBeenCalled()
        })

        it('reports an error on failure', async () => {
            const wrapper = await mountReady()
            mockUpdateEntityDefinition.mockRejectedValue(new Error('conflict'))

            const vm = wrapper.vm as unknown as {
                handleTreeSelection: (items: string[]) => void
                saveChanges: () => Promise<void>
            }
            vm.handleTreeSelection(['def-uuid-1'])
            await wrapper.vm.$nextTick()

            await vm.saveChanges()

            expect(showError).toHaveBeenCalled()
        })
    })

    describe('createEntityDefinition', () => {
        it('creates, reloads the list and closes the dialog', async () => {
            const wrapper = await mountReady()
            const vm = wrapper.vm as unknown as {
                createEntityDefinition: (data: Record<string, unknown>) => Promise<void>
                showCreateDialog: boolean
            }
            vm.showCreateDialog = true
            await wrapper.vm.$nextTick()

            await vm.createEntityDefinition({ entity_type: 'New', display_name: 'New' })

            expect(mockCreateEntityDefinition).toHaveBeenCalled()
            expect(vm.showCreateDialog).toBe(false)
            expect(showSuccess).toHaveBeenCalledWith('Entity definition created successfully')
        })

        it('reports an error on failure', async () => {
            const wrapper = await mountReady()
            mockCreateEntityDefinition.mockRejectedValue(new Error('dup'))

            const vm = wrapper.vm as unknown as {
                createEntityDefinition: (data: Record<string, unknown>) => Promise<void>
            }
            await vm.createEntityDefinition({ entity_type: 'New', display_name: 'New' })

            expect(showError).toHaveBeenCalled()
        })
    })

    describe('editDefinition / updateEntityDefinition', () => {
        it('editDefinition opens the edit dialog', async () => {
            const wrapper = await mountReady()
            const vm = wrapper.vm as unknown as {
                editDefinition: () => void
                showEditDialog: boolean
            }
            vm.editDefinition()
            expect(vm.showEditDialog).toBe(true)
        })

        it('does nothing when there is no selected definition', async () => {
            const wrapper = await mountReady()
            const vm = wrapper.vm as unknown as {
                updateEntityDefinition: (data: Record<string, unknown>) => Promise<void>
            }
            await vm.updateEntityDefinition({ display_name: 'x' })

            expect(mockUpdateEntityDefinition).not.toHaveBeenCalled()
        })

        it('updates, reloads and closes the dialog on success', async () => {
            const wrapper = await mountReady()
            const vm = wrapper.vm as unknown as {
                handleTreeSelection: (items: string[]) => void
                updateEntityDefinition: (data: Record<string, unknown>) => Promise<void>
                showEditDialog: boolean
            }
            vm.handleTreeSelection(['def-uuid-1'])
            vm.showEditDialog = true
            await wrapper.vm.$nextTick()

            await vm.updateEntityDefinition({ display_name: 'Updated' })

            expect(mockUpdateEntityDefinition).toHaveBeenCalledWith('def-uuid-1', {
                display_name: 'Updated',
            })
            expect(vm.showEditDialog).toBe(false)
            expect(showSuccess).toHaveBeenCalledWith('Entity definition updated successfully')
        })

        it('reports an error on failure', async () => {
            const wrapper = await mountReady()
            mockUpdateEntityDefinition.mockRejectedValue(new Error('fail'))

            const vm = wrapper.vm as unknown as {
                handleTreeSelection: (items: string[]) => void
                updateEntityDefinition: (data: Record<string, unknown>) => Promise<void>
            }
            vm.handleTreeSelection(['def-uuid-1'])
            await wrapper.vm.$nextTick()

            await vm.updateEntityDefinition({ display_name: 'Updated' })

            expect(showError).toHaveBeenCalled()
        })
    })

    describe('deleteEntityDefinition', () => {
        it('does nothing when there is no selected definition', async () => {
            const wrapper = await mountReady()
            const vm = wrapper.vm as unknown as { deleteEntityDefinition: () => Promise<void> }
            await vm.deleteEntityDefinition()

            expect(mockDeleteEntityDefinition).not.toHaveBeenCalled()
        })

        it('removes the definition from the list and shows success', async () => {
            const wrapper = await mountReady()
            const vm = wrapper.vm as unknown as {
                handleTreeSelection: (items: string[]) => void
                deleteEntityDefinition: () => Promise<void>
                selectedDefinition: EntityDefinition | null
                showDeleteDialog: boolean
            }
            vm.handleTreeSelection(['def-uuid-1'])
            vm.showDeleteDialog = true
            await wrapper.vm.$nextTick()

            await vm.deleteEntityDefinition()

            expect(mockDeleteEntityDefinition).toHaveBeenCalledWith('def-uuid-1')
            expect(vm.selectedDefinition).toBeNull()
            expect(vm.showDeleteDialog).toBe(false)
            expect(showSuccess).toHaveBeenCalledWith('success')
        })

        it('reports an error on failure', async () => {
            const wrapper = await mountReady()
            mockDeleteEntityDefinition.mockRejectedValue(new Error('in use'))

            const vm = wrapper.vm as unknown as {
                handleTreeSelection: (items: string[]) => void
                deleteEntityDefinition: () => Promise<void>
            }
            vm.handleTreeSelection(['def-uuid-1'])
            await wrapper.vm.$nextTick()

            await vm.deleteEntityDefinition()

            expect(showError).toHaveBeenCalled()
        })
    })

    describe('field editor helpers', () => {
        it('addField opens the editor with no pre-selected field', async () => {
            const wrapper = await mountReady()
            const vm = wrapper.vm as unknown as {
                addField: () => void
                showFieldEditor: boolean
                editingField: FieldDefinition | undefined
            }
            vm.addField()
            expect(vm.showFieldEditor).toBe(true)
            expect(vm.editingField).toBeUndefined()
        })

        it('editField opens the editor with the given field', async () => {
            const wrapper = await mountReady()
            const field = {
                name: 'f1',
                display_name: 'F1',
                field_type: 'String',
            } as unknown as FieldDefinition
            const vm = wrapper.vm as unknown as {
                editField: (f: FieldDefinition) => void
                showFieldEditor: boolean
                editingField: FieldDefinition | undefined
            }
            vm.editField(field)
            expect(vm.showFieldEditor).toBe(true)
            expect(vm.editingField).toEqual(field)
        })

        it('removeField removes the field by name from the selected definition', async () => {
            const wrapper = await mountReady()
            const vm = wrapper.vm as unknown as {
                handleTreeSelection: (items: string[]) => void
                selectedDefinition: EntityDefinition | null
                removeField: (f: FieldDefinition) => void
            }
            vm.handleTreeSelection(['def-uuid-1'])
            await wrapper.vm.$nextTick()
            vm.selectedDefinition!.fields.push({
                name: 'f1',
                display_name: 'F1',
                field_type: 'String',
            } as unknown as FieldDefinition)

            vm.removeField({ name: 'f1' } as unknown as FieldDefinition)

            expect(vm.selectedDefinition!.fields.find(f => f.name === 'f1')).toBeUndefined()
        })

        it('handleFieldSave adds a new field when none is being edited', async () => {
            const wrapper = await mountReady()
            const vm = wrapper.vm as unknown as {
                handleTreeSelection: (items: string[]) => void
                selectedDefinition: EntityDefinition | null
                handleFieldSave: (f: FieldDefinition) => void
            }
            vm.handleTreeSelection(['def-uuid-1'])
            await wrapper.vm.$nextTick()

            vm.handleFieldSave({
                name: 'new_field',
                display_name: 'New',
                field_type: 'String',
            } as unknown as FieldDefinition)

            expect(vm.selectedDefinition!.fields.find(f => f.name === 'new_field')).toBeDefined()
        })

        it('handleFieldSave replaces the field being edited', async () => {
            const wrapper = await mountReady()
            const vm = wrapper.vm as unknown as {
                handleTreeSelection: (items: string[]) => void
                selectedDefinition: EntityDefinition | null
                editField: (f: FieldDefinition) => void
                handleFieldSave: (f: FieldDefinition) => void
            }
            vm.handleTreeSelection(['def-uuid-1'])
            await wrapper.vm.$nextTick()
            vm.selectedDefinition!.fields.push({
                name: 'f1',
                display_name: 'F1',
                field_type: 'String',
            } as unknown as FieldDefinition)
            vm.editField(vm.selectedDefinition!.fields[0])

            vm.handleFieldSave({
                name: 'f1',
                display_name: 'F1 renamed',
                field_type: 'String',
            } as unknown as FieldDefinition)

            expect(vm.selectedDefinition!.fields[0].display_name).toBe('F1 renamed')
        })
    })

    it('opens the create dialog on mount when the route has ?create=true', async () => {
        await router.push('/entity-definitions?create=true')
        await router.isReady()

        const wrapper = mount(EntityDefinitionsPage, { global: { plugins: [router] } })
        await vi.waitUntil(() => mockGetEntityDefinitions.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 10))

        const vm = wrapper.vm as unknown as { showCreateDialog: boolean }
        expect(vm.showCreateDialog).toBe(true)

        await router.push('/entity-definitions')
    })
})
