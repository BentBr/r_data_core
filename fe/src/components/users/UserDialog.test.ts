import { describe, it, expect, vi, beforeEach, afterEach, type Mock } from 'vitest'
import { mount, DOMWrapper, type VueWrapper } from '@vue/test-utils'
import UserDialog from './UserDialog.vue'
import type { UserResponse, Role } from '@/types/schemas'

const mockLoadRoles: Mock = vi.fn()
const mockRoles: Role[] = [
    { uuid: 'role-1', name: 'Admin' } as Role,
    { uuid: 'role-2', name: 'Viewer' } as Role,
]

vi.mock('@/composables/useRoles', () => ({
    useRoles: () => ({
        loadRoles: mockLoadRoles,
        roles: { value: mockRoles },
        loading: { value: false },
    }),
}))

interface DialogProps {
    modelValue: boolean
    editingUser: UserResponse | null
    loading: boolean
}

// v-dialog content is teleported to document.body under real teleport
// behaviour, so DOM assertions need to query the body, not the wrapper —
// and mounts are unmounted afterwards so dialogs don't leak across tests.
let mountedWrappers: VueWrapper[] = []
const mountDialog = (props: DialogProps) => {
    const wrapper = mount(UserDialog, { props, global: { stubs: { teleport: false } } })
    mountedWrappers.push(wrapper)
    return wrapper
}
const body = () => new DOMWrapper(document.body)

const makeUser = (overrides: Partial<UserResponse> = {}): UserResponse =>
    ({
        uuid: 'user-1',
        username: 'bob',
        email: 'bob@example.com',
        first_name: 'Bob',
        last_name: 'Builder',
        role_uuids: ['role-1'],
        is_active: true,
        is_admin: false,
        super_admin: false,
        is_sso_provisioned: false,
        status: 'active',
        last_login: null,
        failed_login_attempts: 0,
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        created_by: 'user-0',
        ...overrides,
    }) as UserResponse

describe('UserDialog', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        mockLoadRoles.mockResolvedValue(undefined)
    })

    afterEach(() => {
        mountedWrappers.forEach(w => w.unmount())
        mountedWrappers = []
    })

    it('loads the available roles on mount', async () => {
        mountDialog({ modelValue: true, editingUser: null, loading: false })
        await Promise.resolve()

        expect(mockLoadRoles).toHaveBeenCalledWith(1, 100)
    })

    it('shows create-mode defaults when there is no editing user', () => {
        const wrapper = mountDialog({ modelValue: true, editingUser: null, loading: false })

        expect((wrapper.vm as any).formData.username).toBe('')
        expect((wrapper.vm as any).formData.is_active).toBe(true)
        expect((wrapper.vm as any).formData.super_admin).toBe(false)
    })

    it('populates the form from the editing user', () => {
        const user = makeUser()
        const wrapper = mountDialog({ modelValue: true, editingUser: user, loading: false })

        expect((wrapper.vm as any).formData.username).toBe('bob')
        expect((wrapper.vm as any).formData.email).toBe('bob@example.com')
        expect((wrapper.vm as any).formData.role_uuids).toEqual(['role-1'])
        expect((wrapper.vm as any).formData.password).toBe('')
    })

    it('resets the form when switching back to create mode', async () => {
        const user = makeUser()
        const wrapper = mountDialog({ modelValue: true, editingUser: user, loading: false })

        await wrapper.setProps({ editingUser: null })

        expect((wrapper.vm as any).formData.username).toBe('')
        expect((wrapper.vm as any).formData.email).toBe('')
    })

    it('treats a federated account as an SSO user', () => {
        const user = makeUser({ is_sso_provisioned: true })
        const wrapper = mountDialog({ modelValue: true, editingUser: user, loading: false })

        expect((wrapper.vm as any).isSsoUser).toBe(true)
        expect(body().find('[data-testid="sso-roles-readonly"]').exists()).toBe(true)
    })

    it('does not treat a local account as an SSO user', () => {
        const user = makeUser({ is_sso_provisioned: false })
        const wrapper = mountDialog({ modelValue: true, editingUser: user, loading: false })

        expect((wrapper.vm as any).isSsoUser).toBe(false)
        expect(body().find('[data-testid="sso-roles-readonly"]').exists()).toBe(false)
    })

    it('resolves assigned role names for an SSO user from the available roles', async () => {
        const user = makeUser({ is_sso_provisioned: true, role_uuids: ['role-1', 'role-2'] })
        const wrapper = mountDialog({ modelValue: true, editingUser: user, loading: false })
        await Promise.resolve()
        await Promise.resolve()

        expect((wrapper.vm as any).assignedRoleNames).toEqual(['Admin', 'Viewer'])
    })

    it('shows no roles message when an SSO user has none assigned', () => {
        const user = makeUser({ is_sso_provisioned: true, role_uuids: [] })
        const wrapper = mountDialog({ modelValue: true, editingUser: user, loading: false })

        expect((wrapper.vm as any).assignedRoleNames).toEqual([])
        expect(body().find('[data-testid="sso-roles-readonly"]').text()).toContain('sso_roles_none')
    })

    it('reloads roles when the dialog is reopened', async () => {
        const wrapper = mountDialog({ modelValue: false, editingUser: null, loading: false })
        mockLoadRoles.mockClear()

        await wrapper.setProps({ modelValue: true })

        expect(mockLoadRoles).toHaveBeenCalledWith(1, 100)
    })

    it('resets the form when the dialog is closed', async () => {
        const wrapper = mountDialog({ modelValue: true, editingUser: null, loading: false })
        ;(wrapper.vm as any).formData.email = 'typed@example.com'
        await wrapper.vm.$nextTick()

        await wrapper.setProps({ modelValue: false })

        expect((wrapper.vm as any).formData.email).toBe('')
    })

    it('emits update:modelValue(false) and resets the form on cancel', async () => {
        const wrapper = mountDialog({ modelValue: true, editingUser: null, loading: false })
        ;(wrapper.vm as any).formData.email = 'typed@example.com'

        ;(wrapper.vm as any).handleClose()

        expect(wrapper.emitted('update:modelValue')?.[0]).toEqual([false])
        expect((wrapper.vm as any).formData.email).toBe('')
    })

    it('does not emit save when the form is invalid', () => {
        const wrapper = mountDialog({ modelValue: true, editingUser: null, loading: false })
        ;(wrapper.vm as any).formValid = false

        ;(wrapper.vm as any).handleSave()

        expect(wrapper.emitted('save')).toBeFalsy()
    })

    it('emits a create payload for a new user when valid', () => {
        const wrapper = mountDialog({ modelValue: true, editingUser: null, loading: false })
        const vm = wrapper.vm as any
        vm.formValid = true
        vm.formData.username = 'newuser'
        vm.formData.email = 'new@example.com'
        vm.formData.password = 'secret123'
        vm.formData.first_name = 'New'
        vm.formData.last_name = 'User'
        vm.formData.role_uuids = ['role-1']

        vm.handleSave()

        expect(wrapper.emitted('save')?.[0]?.[0]).toEqual({
            username: 'newuser',
            email: 'new@example.com',
            password: 'secret123',
            first_name: 'New',
            last_name: 'User',
            role_uuids: ['role-1'],
            is_active: true,
            super_admin: false,
        })
    })

    it('emits an update payload without a password when left blank while editing', () => {
        const user = makeUser()
        const wrapper = mountDialog({ modelValue: true, editingUser: user, loading: false })
        const vm = wrapper.vm as any
        vm.formValid = true
        vm.formData.password = ''

        vm.handleSave()

        const payload = wrapper.emitted('save')?.[0]?.[0] as Record<string, unknown>
        expect(payload).not.toHaveProperty('password')
        expect(payload.email).toBe('bob@example.com')
    })

    it('includes the password in the update payload when provided', () => {
        const user = makeUser()
        const wrapper = mountDialog({ modelValue: true, editingUser: user, loading: false })
        const vm = wrapper.vm as any
        vm.formValid = true
        vm.formData.password = 'newpassword123'

        vm.handleSave()

        const payload = wrapper.emitted('save')?.[0]?.[0] as Record<string, unknown>
        expect(payload.password).toBe('newpassword123')
    })

    it('validates required fields', () => {
        const wrapper = mountDialog({ modelValue: true, editingUser: null, loading: false })
        const vm = wrapper.vm as any

        expect(vm.rules.required('')).not.toBe(true)
        expect(vm.rules.required('value')).toBe(true)
    })

    it('validates email format', () => {
        const wrapper = mountDialog({ modelValue: true, editingUser: null, loading: false })
        const vm = wrapper.vm as any

        expect(vm.rules.email('')).toBe(true)
        expect(vm.rules.email('not-an-email')).not.toBe(true)
        expect(vm.rules.email('ok@example.com')).toBe(true)
    })

    it('validates minimum length', () => {
        const wrapper = mountDialog({ modelValue: true, editingUser: null, loading: false })
        const vm = wrapper.vm as any
        const minLength8 = vm.rules.minLength(8)

        expect(minLength8('')).toBe(true)
        expect(minLength8('short')).not.toBe(true)
        expect(minLength8('longenough')).toBe(true)
    })

    it('logs and continues when loading roles fails', async () => {
        mockLoadRoles.mockRejectedValue(new Error('network down'))
        const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

        mountDialog({ modelValue: true, editingUser: null, loading: false })
        await Promise.resolve()
        await Promise.resolve()

        expect(consoleErrorSpy).toHaveBeenCalledWith('Failed to load roles:', expect.any(Error))
        consoleErrorSpy.mockRestore()
    })
})
