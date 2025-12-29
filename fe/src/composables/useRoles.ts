import { ref } from 'vue'
import { typedHttpClient } from '@/api/typed-client'
import { useErrorHandler } from '@/composables/useErrorHandler'
import { useTranslations } from '@/composables/useTranslations'
import type { Role, CreateRoleRequest, UpdateRoleRequest } from '@/types/schemas'

export function useRoles() {
    const { handleError, handleSuccess } = useErrorHandler()
    const { t } = useTranslations()

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
            handleError(err)
            throw err
        } finally {
            loading.value = false
        }
    }

    const createRole = async (data: CreateRoleRequest) => {
        try {
            await typedHttpClient.createRole(data)
            handleSuccess(t('roles.create.success'))
        } catch (err) {
            handleError(err)
            throw err
        }
    }

    const updateRole = async (uuid: string, data: UpdateRoleRequest) => {
        try {
            await typedHttpClient.updateRole(uuid, data)
            handleSuccess(t('roles.update.success'))
        } catch (err) {
            handleError(err)
            throw err
        }
    }

    const deleteRole = async (uuid: string) => {
        try {
            await typedHttpClient.deleteRole(uuid)
            handleSuccess(t('roles.delete.success'))
        } catch (err) {
            handleError(err)
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
