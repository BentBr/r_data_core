import { ref } from 'vue'
import { typedHttpClient } from '@/api/typed-client'
import { useSnackbar } from '@/composables/useSnackbar'
import type { Role, CreateRoleRequest, UpdateRoleRequest } from '@/types/schemas'

export function useRoles() {
    const { showSuccess, showError } = useSnackbar()

    const loading = ref(false)
    const error = ref('')
    const roles = ref<Role[]>([])

    const loadRoles = async (
        page = 1,
        perPage = 20,
        sortBy?: string | null,
        sortOrder?: 'asc' | 'desc' | null
    ) => {
        loading.value = true
        error.value = ''

        try {
            const response = await typedHttpClient.getRoles(page, perPage, sortBy, sortOrder)
            roles.value = response.data
            return response
        } catch (err) {
            console.error('Failed to load roles:', err)
            const errorMessage = err instanceof Error ? err.message : 'Failed to load roles'
            error.value = errorMessage
            showError(errorMessage)
            throw err
        } finally {
            loading.value = false
        }
    }

    const createRole = async (data: CreateRoleRequest) => {
        try {
            await typedHttpClient.createRole(data)
            showSuccess('Role created successfully')
        } catch (err) {
            const message = err instanceof Error ? err.message : 'Failed to create role'
            showError(message)
            throw err
        }
    }

    const updateRole = async (uuid: string, data: UpdateRoleRequest) => {
        try {
            await typedHttpClient.updateRole(uuid, data)
            showSuccess('Role updated successfully')
        } catch (err) {
            const message = err instanceof Error ? err.message : 'Failed to update role'
            showError(message)
            throw err
        }
    }

    const deleteRole = async (uuid: string) => {
        try {
            await typedHttpClient.deleteRole(uuid)
            showSuccess('Role deleted successfully')
        } catch (err) {
            const message = err instanceof Error ? err.message : 'Failed to delete role'
            showError(message)
            throw err
        }
    }

    return {
        loading,
        error,
        roles,
        loadRoles,
        createRole,
        updateRole,
        deleteRole,
    }
}
