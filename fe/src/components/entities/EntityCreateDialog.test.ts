import { mount, VueWrapper } from '@vue/test-utils'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import EntityCreateDialog from './EntityCreateDialog.vue'
import { createVuetify } from 'vuetify'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'

const vuetify = createVuetify({ components, directives })

// Mock API
vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        searchEntitiesByPath: vi.fn(),
        browseByPath: vi.fn(),
        getEntity: vi.fn(),
    },
    ValidationError: class ValidationError extends Error {
        violations: Array<{ field: string; message: string }>
        constructor(violations: Array<{ field: string; message: string }>) {
            super('validation')
            this.violations = violations
        }
    },
}))

// Mock Translations
vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({
        t: (key: string, params?: Record<string, string>) => {
            const translations: Record<string, string> = {
                'entities.create.root_folder_hint': 'This is your root folder',
                'entities.create.path_hint': '/ for root or /folder/subfolder',
                'entities.create.new_folder_hint': 'You are creating a new folder {folder}',
                'entities.create.path_label': 'Path',
                'entities.create.parent_label': 'Parent Entity (Optional)',
                'entities.create.published_label': 'Published',
                'entities.create.title': 'Create New Entity',
                'entities.create.entity_type_label': 'Entity Type',
                'entities.create.key_label': 'Key',
                'entities.create.create_button': 'Create',
                'common.cancel': 'Cancel',
                'table.loading': 'Loading...',
                'entities.tree.no_entities': 'No entities found',
            }
            let result = translations[key] || key
            // Replace {param} placeholders with actual values
            if (params) {
                for (const [paramKey, paramValue] of Object.entries(params)) {
                    result = result.replace(`{${paramKey}}`, paramValue)
                }
            }
            return result
        },
    }),
}))

// Mock SmartIcon
vi.mock('@/components/common/SmartIcon.vue', () => ({
    default: { template: '<div class="smart-icon"></div>' },
}))

// Mock useFieldRendering
vi.mock('@/composables/useFieldRendering', () => ({
    useFieldRendering: () => ({
        getFieldComponent: () => 'v-text-field',
        getFieldRules: () => [],
        parseJsonFieldValue: (value: unknown, fieldType: string) => {
            if (fieldType !== 'Json') {
                return { parsed: value, error: null }
            }
            if (typeof value !== 'string' || value.trim() === '') {
                return { parsed: value, error: null }
            }
            try {
                return { parsed: JSON.parse(value), error: null }
            } catch {
                return { parsed: value, error: 'Invalid JSON' }
            }
        },
    }),
}))

// Mock design system
vi.mock('@/design-system/components', () => ({
    getDialogMaxWidth: () => '600px',
    buttonConfigs: {
        text: { variant: 'text' },
        primary: { color: 'primary', variant: 'flat' },
    },
}))

describe('EntityCreateDialog', () => {
    let wrapper: VueWrapper

    beforeEach(() => {
        vi.clearAllMocks()
        vi.useFakeTimers()
    })

    afterEach(() => {
        vi.useRealTimers()
        wrapper.unmount()
    })

    const mountComponent = (props = {}) => {
        return mount(EntityCreateDialog, {
            global: { plugins: [vuetify] },
            props: {
                modelValue: true,
                entityDefinitions: [
                    {
                        uuid: 'def-uuid-1',
                        entity_type: 'test_type',
                        display_name: 'Test Type',
                        published: true,
                        allow_children: true,
                        fields: [],
                        created_at: '2024-01-01T00:00:00Z',
                        updated_at: '2024-01-01T00:00:00Z',
                        created_by: 'user-uuid',
                        version: 1,
                    },
                ] as any,
                ...props,
            },
        })
    }

    describe('Path hint computed property', () => {
        it('returns root folder hint when path is "/"', () => {
            wrapper = mountComponent()

            const vm = wrapper.vm as unknown as {
                isRootPath: boolean
                pathHint: string
                formData: { data: { path: string } }
            }

            // Default path is "/"
            expect(vm.formData.data.path).toBe('/')
            expect(vm.isRootPath).toBe(true)
            expect(vm.pathHint).toBe('This is your root folder')
        })

        it('returns default hint when path matches existing entity', async () => {
            wrapper = mountComponent()

            const vm = wrapper.vm as unknown as {
                isRootPath: boolean
                pathHint: string
                formData: { data: { path: string } }
                pathSuggestions: Array<{
                    kind: string
                    name: string
                    path: string
                    entity_uuid: string
                }>
            }

            // Set path that matches an entity
            vm.pathSuggestions = [
                {
                    kind: 'file',
                    name: 'existing',
                    path: '/existing/path',
                    entity_uuid: 'uuid-123',
                },
            ]
            vm.formData.data.path = '/existing/path'
            await wrapper.vm.$nextTick()

            expect(vm.isRootPath).toBe(false)
            expect(vm.pathHint).toBe('/ for root or /folder/subfolder')
        })

        it('returns new folder hint when path does not match any entity', async () => {
            wrapper = mountComponent()

            const vm = wrapper.vm as unknown as {
                isRootPath: boolean
                isCreatingNewFolder: boolean
                newFolderName: string
                pathHint: string
                formData: { data: { path: string } }
                pathSuggestions: Array<unknown>
            }

            // Set path that does not match any entity
            vm.pathSuggestions = []
            vm.formData.data.path = '/my-new-folder'
            await wrapper.vm.$nextTick()

            expect(vm.isRootPath).toBe(false)
            expect(vm.isCreatingNewFolder).toBe(true)
            expect(vm.newFolderName).toBe('my-new-folder')
            expect(vm.pathHint).toBe('You are creating a new folder my-new-folder')
        })

        it('extracts correct folder name from nested path', async () => {
            wrapper = mountComponent()

            const vm = wrapper.vm as unknown as {
                newFolderName: string
                pathHint: string
                formData: { data: { path: string } }
                pathSuggestions: Array<unknown>
            }

            vm.pathSuggestions = []
            vm.formData.data.path = '/parent/child/grandchild'
            await wrapper.vm.$nextTick()

            expect(vm.newFolderName).toBe('grandchild')
            expect(vm.pathHint).toBe('You are creating a new folder grandchild')
        })

        it('handles path with trailing slash', async () => {
            wrapper = mountComponent()

            const vm = wrapper.vm as unknown as {
                newFolderName: string
                formData: { data: { path: string } }
                pathSuggestions: Array<unknown>
            }

            vm.pathSuggestions = []
            vm.formData.data.path = '/my-folder/'
            await wrapper.vm.$nextTick()

            expect(vm.newFolderName).toBe('my-folder')
        })
    })

    describe('Custom path input', () => {
        it('allows custom path input that does not match any entity', async () => {
            wrapper = mountComponent()

            const vm = wrapper.vm as unknown as {
                formData: { data: { path: string }; parent_uuid: string | null }
                pathSuggestions: Array<unknown>
                onPathSelected: (value: unknown) => void
            }

            // Simulate user entering a custom path
            vm.formData.data.path = '/my-custom-folder'
            vm.pathSuggestions = [] // No matching entities

            // Trigger selection (simulating blur/selection)
            vm.onPathSelected('/my-custom-folder')
            await wrapper.vm.$nextTick()

            // Path should be preserved
            expect(vm.formData.data.path).toBe('/my-custom-folder')
            // Parent should remain null since no entity matches
            expect(vm.formData.parent_uuid).toBeNull()
        })

        it('preserves custom nested path', async () => {
            wrapper = mountComponent()

            const vm = wrapper.vm as unknown as {
                formData: { data: { path: string } }
                pathSuggestions: Array<unknown>
                onPathSelected: (value: unknown) => void
            }

            vm.formData.data.path = '/products/electronics/phones'
            vm.pathSuggestions = []

            vm.onPathSelected('/products/electronics/phones')
            await wrapper.vm.$nextTick()

            expect(vm.formData.data.path).toBe('/products/electronics/phones')
        })
    })

    describe('Path autocomplete debouncing', () => {
        it('debounces path search with 350ms delay', async () => {
            const { typedHttpClient } = await import('@/api/typed-client')
            vi.mocked(typedHttpClient.searchEntitiesByPath).mockResolvedValue({
                data: [],
            })

            wrapper = mountComponent()

            const vm = wrapper.vm as unknown as {
                onPathInput: (value: string | null) => void
            }

            // Trigger multiple path inputs
            vm.onPathInput('/test1')
            vm.onPathInput('/test2')
            vm.onPathInput('/test3')

            // Should not have called API yet
            expect(typedHttpClient.searchEntitiesByPath).not.toHaveBeenCalled()

            // Advance timer past debounce delay
            vi.advanceTimersByTime(350)
            await wrapper.vm.$nextTick()

            // Should have called API once (with last value)
            expect(typedHttpClient.searchEntitiesByPath).toHaveBeenCalledTimes(1)
            expect(typedHttpClient.searchEntitiesByPath).toHaveBeenCalledWith('/test3', 10)
        })

        it('does not search for root path "/"', async () => {
            const { typedHttpClient } = await import('@/api/typed-client')

            wrapper = mountComponent()

            const vm = wrapper.vm as unknown as {
                onPathInput: (value: string | null) => void
            }

            vm.onPathInput('/')

            vi.advanceTimersByTime(350)
            await wrapper.vm.$nextTick()

            expect(typedHttpClient.searchEntitiesByPath).not.toHaveBeenCalled()
        })

        it('clears suggestions when path is empty', () => {
            wrapper = mountComponent()

            const vm = wrapper.vm as unknown as {
                onPathInput: (value: string | null) => void
                pathSuggestions: Array<unknown>
            }

            // Set some suggestions first
            vm.pathSuggestions = [{ kind: 'file', name: 'test', path: '/test' }]

            // Clear path
            vm.onPathInput('')

            expect(vm.pathSuggestions).toEqual([])
        })
    })

    describe('Parent selection', () => {
        it('sets parent when path suggestion is selected', () => {
            wrapper = mountComponent()

            const vm = wrapper.vm as unknown as {
                pathSuggestions: Array<{
                    kind: string
                    name: string
                    path: string
                    entity_uuid: string
                }>
                onPathSelected: (value: unknown) => void
                formData: { parent_uuid: string | null }
            }

            // Set suggestions
            vm.pathSuggestions = [
                {
                    kind: 'file',
                    name: 'entity1',
                    path: '/folder/entity1',
                    entity_uuid: 'uuid-123',
                },
            ]

            // Select the path
            vm.onPathSelected('/folder/entity1')

            expect(vm.formData.parent_uuid).toBe('uuid-123')
        })

        it('updates path when parent is selected', () => {
            wrapper = mountComponent()

            const vm = wrapper.vm as unknown as {
                parentSuggestions: Array<{
                    kind: string
                    name: string
                    path: string
                    entity_uuid: string
                }>
                onParentSelect: (uuid: string | null) => void
                formData: { data: { path: string }; parent_uuid: string | null }
            }

            // Set parent suggestions
            vm.parentSuggestions = [
                {
                    kind: 'file',
                    name: 'parent1',
                    path: '/folder/parent1',
                    entity_uuid: 'uuid-parent',
                },
            ]

            // Select parent
            vm.onParentSelect('uuid-parent')

            expect(vm.formData.parent_uuid).toBe('uuid-parent')
            expect(vm.formData.data.path).toBe('/folder/parent1')
        })

        it('loads entities at current path on parent dropdown click', async () => {
            const { typedHttpClient } = await import('@/api/typed-client')
            vi.mocked(typedHttpClient.browseByPath).mockResolvedValue({
                data: [
                    {
                        kind: 'file',
                        name: 'entity1',
                        path: '/entity1',
                        entity_uuid: 'uuid-1',
                        entity_type: 'test_type',
                        published: true,
                    },
                ],
            })

            wrapper = mountComponent()

            const vm = wrapper.vm as unknown as {
                formData: { data: { path: string } }
                onParentDropdownClick: () => Promise<void>
            }

            vm.formData.data.path = '/'

            await vm.onParentDropdownClick()

            expect(typedHttpClient.browseByPath).toHaveBeenCalledWith('/', 10)
        })
    })

    describe('Parent search debouncing', () => {
        it('debounces parent search with 350ms delay', async () => {
            const { typedHttpClient } = await import('@/api/typed-client')
            vi.mocked(typedHttpClient.searchEntitiesByPath).mockResolvedValue({
                data: [],
            })

            wrapper = mountComponent()

            const vm = wrapper.vm as unknown as {
                onParentSearch: (value: string | null) => void
            }

            // Trigger multiple searches
            vm.onParentSearch('test1')
            vm.onParentSearch('test2')

            // Should not have called API yet
            expect(typedHttpClient.searchEntitiesByPath).not.toHaveBeenCalled()

            // Advance timer
            vi.advanceTimersByTime(350)
            await wrapper.vm.$nextTick()

            // Should have called API once
            expect(typedHttpClient.searchEntitiesByPath).toHaveBeenCalledTimes(1)
        })
    })

    describe('pathSetByParent flag', () => {
        it('does not clear parent when path is set by parent selection', () => {
            wrapper = mountComponent()

            const vm = wrapper.vm as unknown as {
                parentSuggestions: Array<{
                    kind: string
                    name: string
                    path: string
                    entity_uuid: string
                }>
                onParentSelect: (uuid: string | null) => void
                formData: { parent_uuid: string | null; data: { path: string } }
                pathSetByParent: boolean
            }

            // Set parent suggestions
            vm.parentSuggestions = [
                {
                    kind: 'file',
                    name: 'parent1',
                    path: '/folder/parent1',
                    entity_uuid: 'uuid-parent',
                },
            ]

            // Select parent (this sets pathSetByParent=true and updates path)
            vm.onParentSelect('uuid-parent')

            // The parent should still be set
            expect(vm.formData.parent_uuid).toBe('uuid-parent')
        })
    })

    describe('Error handling', () => {
        it('clears suggestions on API error for parent dropdown', async () => {
            const { typedHttpClient } = await import('@/api/typed-client')
            vi.mocked(typedHttpClient.browseByPath).mockRejectedValue(new Error('API Error'))

            wrapper = mountComponent()

            const vm = wrapper.vm as unknown as {
                onParentDropdownClick: () => Promise<void>
                parentSuggestions: Array<unknown>
                parentLoading: boolean
            }

            await vm.onParentDropdownClick()

            expect(vm.parentSuggestions).toEqual([])
            expect(vm.parentLoading).toBe(false)
        })
    })

    describe('selectedParentDisplay', () => {
        it('sets display text when parent is selected via path', () => {
            wrapper = mountComponent()

            const vm = wrapper.vm as unknown as {
                pathSuggestions: Array<{
                    kind: string
                    name: string
                    path: string
                    entity_uuid: string
                }>
                onPathSelected: (value: unknown) => void
                selectedParentDisplay: string | null
            }

            // Set suggestions
            vm.pathSuggestions = [
                {
                    kind: 'file',
                    name: 'entity1',
                    path: '/folder/entity1',
                    entity_uuid: 'uuid-123',
                },
            ]

            // Select the path
            vm.onPathSelected('/folder/entity1')

            // Should have set display text to path
            expect(vm.selectedParentDisplay).toBe('/folder/entity1')
        })

        it('sets display text when parent is selected via parent dropdown', () => {
            wrapper = mountComponent()

            const vm = wrapper.vm as unknown as {
                parentSuggestions: Array<{
                    kind: string
                    name: string
                    path: string
                    entity_uuid: string
                }>
                onParentSelect: (uuid: string | null) => void
                selectedParentDisplay: string | null
            }

            // Set parent suggestions
            vm.parentSuggestions = [
                {
                    kind: 'file',
                    name: 'parent1',
                    path: '/folder/parent1',
                    entity_uuid: 'uuid-parent',
                },
            ]

            // Select parent
            vm.onParentSelect('uuid-parent')

            // Should have set display text to path
            expect(vm.selectedParentDisplay).toBe('/folder/parent1')
        })

        it('clears display text when parent is cleared', () => {
            wrapper = mountComponent()

            const vm = wrapper.vm as unknown as {
                parentSuggestions: Array<{
                    kind: string
                    name: string
                    path: string
                    entity_uuid: string
                }>
                onParentSelect: (uuid: string | null) => void
                selectedParentDisplay: string | null
            }

            // Set initial parent
            vm.parentSuggestions = [
                {
                    kind: 'file',
                    name: 'parent1',
                    path: '/folder/parent1',
                    entity_uuid: 'uuid-parent',
                },
            ]
            vm.onParentSelect('uuid-parent')
            expect(vm.selectedParentDisplay).toBe('/folder/parent1')

            // Clear parent
            vm.onParentSelect(null)

            // Should have cleared display text
            expect(vm.selectedParentDisplay).toBeNull()
        })

        it('clears display text when parent search yields no match', () => {
            wrapper = mountComponent()

            const vm = wrapper.vm as unknown as {
                onParentSelect: (uuid: string | null) => void
                selectedParentDisplay: string | null
            }

            vm.onParentSelect(null)

            expect(vm.selectedParentDisplay).toBeNull()
        })

        it('preserves display text when path suggestions change', () => {
            wrapper = mountComponent()

            const vm = wrapper.vm as unknown as {
                parentSuggestions: Array<{
                    kind: string
                    name: string
                    path: string
                    entity_uuid: string
                }>
                onParentSelect: (uuid: string | null) => void
                selectedParentDisplay: string | null
                formData: { parent_uuid: string | null }
            }

            // Set parent
            vm.parentSuggestions = [
                {
                    kind: 'file',
                    name: 'parent1',
                    path: '/folder/parent1',
                    entity_uuid: 'uuid-parent',
                },
            ]
            vm.onParentSelect('uuid-parent')

            // Clear suggestions (simulating what happens when path changes)
            vm.parentSuggestions = []

            // Display text should still be set
            expect(vm.selectedParentDisplay).toBe('/folder/parent1')
            expect(vm.formData.parent_uuid).toBe('uuid-parent')
        })
    })

    describe('onEntityTypeChange', () => {
        it('resets form data and applies ui_settings defaults for the new type', () => {
            wrapper = mountComponent({
                entityDefinitions: [
                    {
                        uuid: 'def-uuid-2',
                        entity_type: 'with_defaults',
                        display_name: 'With Defaults',
                        published: true,
                        allow_children: true,
                        fields: [
                            {
                                name: 'status',
                                display_name: 'Status',
                                field_type: 'String',
                                ui_settings: { default: 'active' },
                            },
                        ],
                        created_at: '2024-01-01T00:00:00Z',
                        updated_at: '2024-01-01T00:00:00Z',
                        created_by: 'user-uuid',
                        version: 1,
                    },
                ] as any,
            })

            const vm = wrapper.vm as unknown as {
                formData: {
                    entity_type: string
                    data: Record<string, unknown>
                    parent_uuid: string | null
                }
                onEntityTypeChange: () => void
                selectedParentDisplay: string | null
            }

            vm.formData.entity_type = 'with_defaults'
            vm.formData.parent_uuid = 'some-parent'
            vm.onEntityTypeChange()

            expect(vm.formData.data.entity_key).toBe('')
            expect(vm.formData.data.path).toBe('/')
            expect(vm.formData.data.published).toBe(false)
            expect(vm.formData.data.status).toBe('active')
            expect(vm.formData.parent_uuid).toBeNull()
            expect(vm.selectedParentDisplay).toBeNull()
        })
    })

    describe('closeDialog', () => {
        it('resets all form and suggestion state', async () => {
            wrapper = mountComponent()

            const vm = wrapper.vm as unknown as {
                formData: {
                    entity_type: string
                    data: Record<string, unknown>
                    parent_uuid: string | null
                }
                pathSuggestions: unknown[]
                parentSuggestions: unknown[]
                selectedParentDisplay: string | null
                fieldErrors: Record<string, string>
                closeDialog: () => void
            }

            vm.formData.entity_type = 'test_type'
            vm.formData.data.entity_key = 'some-key'
            vm.pathSuggestions = [{ kind: 'file', name: 'a', path: '/a' }]
            vm.parentSuggestions = [{ kind: 'file', name: 'b', path: '/b' }]
            vm.selectedParentDisplay = '/b'
            vm.fieldErrors = { entity_key: 'taken' }

            vm.closeDialog()
            await wrapper.vm.$nextTick()

            expect(vm.formData.entity_type).toBe('')
            expect(vm.formData.data).toEqual({
                entity_key: '',
                path: '/',
                published: false,
            })
            expect(vm.pathSuggestions).toEqual([])
            expect(vm.parentSuggestions).toEqual([])
            expect(vm.selectedParentDisplay).toBeNull()
            expect(vm.fieldErrors).toEqual({})
        })

        it('is triggered when the dialog visibility watcher turns false', async () => {
            wrapper = mountComponent()

            const vm = wrapper.vm as unknown as {
                formData: { entity_type: string }
            }
            vm.formData.entity_type = 'test_type'

            await wrapper.setProps({ modelValue: false })
            await wrapper.vm.$nextTick()

            expect(vm.formData.entity_type).toBe('')
        })
    })

    describe('defaultParent prefill watchers', () => {
        it('prefills parent and path when defaultParent changes while dialog is open', async () => {
            wrapper = mountComponent({ modelValue: true })

            const vm = wrapper.vm as unknown as {
                formData: {
                    entity_type: string
                    parent_uuid: string | null
                    data: { path: string }
                }
                selectedParentDisplay: string | null
            }
            vm.formData.entity_type = 'test_type'

            await wrapper.setProps({
                defaultParent: {
                    entity_type: 'test_type',
                    field_data: {
                        uuid: 'parent-uuid-1',
                        path: '/parent-folder',
                        entity_key: 'parent-key',
                    },
                } as any,
            })
            await wrapper.vm.$nextTick()

            expect(vm.formData.parent_uuid).toBe('parent-uuid-1')
            expect(vm.formData.data.path).toBe('/parent-folder/parent-key')
            expect(vm.selectedParentDisplay).toBe('/parent-folder/parent-key')
        })

        it('does not prefill when entity_type has not been selected yet', async () => {
            wrapper = mountComponent({ modelValue: true })

            const vm = wrapper.vm as unknown as {
                formData: { parent_uuid: string | null }
            }

            await wrapper.setProps({
                defaultParent: {
                    entity_type: 'test_type',
                    field_data: {
                        uuid: 'parent-uuid-1',
                        path: '/parent-folder',
                        entity_key: 'parent-key',
                    },
                } as any,
            })
            await wrapper.vm.$nextTick()

            expect(vm.formData.parent_uuid).toBeNull()
        })

        it('prefills from an already-set defaultParent when dialog opens', async () => {
            wrapper = mountComponent({
                modelValue: false,
                defaultParent: {
                    entity_type: 'test_type',
                    field_data: {
                        uuid: 'parent-uuid-2',
                        path: '/root',
                        entity_key: 'child',
                    },
                } as any,
            })

            const vm = wrapper.vm as unknown as {
                formData: {
                    entity_type: string
                    parent_uuid: string | null
                    data: { path: string }
                }
            }
            vm.formData.entity_type = 'test_type'

            await wrapper.setProps({ modelValue: true })
            await wrapper.vm.$nextTick()

            expect(vm.formData.parent_uuid).toBe('parent-uuid-2')
            expect(vm.formData.data.path).toBe('/root/child')
        })
    })

    describe('createEntity submission', () => {
        // form.value is a real VForm ref, but exercising its actual
        // validate() would depend on Vuetify's internal (debounced/RAF-based)
        // validation pipeline, which is unreliable under vi.useFakeTimers().
        // We stub `form` directly with a fake validate() so these tests
        // exercise createEntity's own logic deterministically. Note this also
        // demonstrates a production bug (see "never blocks" test below):
        // `if (!form.value?.validate())` never awaits the Promise
        // VForm.validate() returns, so the guard is always a no-op.
        const withFakeForm = (w: VueWrapper, resolvedValue: unknown = { valid: true }) => {
            ;(w.vm as unknown as { form: unknown }).form = {
                validate: () => Promise.resolve(resolvedValue),
            }
        }

        it('emits create with processed data on success', async () => {
            wrapper = mountComponent()
            withFakeForm(wrapper)

            const vm = wrapper.vm as unknown as {
                formData: { entity_type: string; data: Record<string, unknown> }
                createEntity: () => Promise<void>
            }
            vm.formData.entity_type = 'test_type'
            vm.formData.data.entity_key = 'my-key'

            await vm.createEntity()

            expect(wrapper.emitted('create')).toBeTruthy()
            const payload = wrapper.emitted('create')?.[0]?.[0] as {
                entity_type: string
                data: Record<string, unknown>
            }
            expect(payload.entity_type).toBe('test_type')
            expect(payload.data.entity_key).toBe('my-key')
        })

        it('sets a field error and skips emit when a JSON field is invalid', async () => {
            wrapper = mountComponent({
                entityDefinitions: [
                    {
                        uuid: 'def-uuid-3',
                        entity_type: 'json_type',
                        display_name: 'Json Type',
                        published: true,
                        allow_children: true,
                        fields: [{ name: 'meta', display_name: 'Meta', field_type: 'Json' }],
                        created_at: '2024-01-01T00:00:00Z',
                        updated_at: '2024-01-01T00:00:00Z',
                        created_by: 'user-uuid',
                        version: 1,
                    },
                ] as any,
            })
            withFakeForm(wrapper)

            const vm = wrapper.vm as unknown as {
                formData: { entity_type: string; data: Record<string, unknown> }
                createEntity: () => Promise<void>
                getFieldErrorMessages: (name: string) => string[]
            }
            vm.formData.entity_type = 'json_type'
            vm.formData.data.meta = '{not valid json'
            // Let the deep watcher on formData.data (which clears stale field
            // errors) settle before invoking createEntity, otherwise it can
            // race with and wipe out the error createEntity is about to set.
            await wrapper.vm.$nextTick()

            await vm.createEntity()

            expect(wrapper.emitted('create')).toBeFalsy()
            expect(vm.getFieldErrorMessages('meta').length).toBeGreaterThan(0)
        })

        it('never blocks submission on an invalid form (missing await on validate())', async () => {
            // Production bug: form.value.validate() returns a Promise, and
            // `!promise` is always false, so an invalid form never
            // short-circuits createEntity. Simulate Vuetify reporting the
            // form as invalid and show that it still emits.
            wrapper = mountComponent()
            withFakeForm(wrapper, { valid: false, errors: [{ id: 'entity_type' }] })

            const vm = wrapper.vm as unknown as {
                formData: { entity_type: string; data: Record<string, unknown> }
                createEntity: () => Promise<void>
            }
            vm.formData.entity_type = 'test_type'
            vm.formData.data.entity_key = 'my-key'

            await vm.createEntity()

            expect(wrapper.emitted('create')).toBeTruthy()
        })
    })
})
