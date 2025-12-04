import { ref } from 'vue'
import { typedHttpClient } from '@/api/typed-client'
import { useSnackbar } from '@/composables/useSnackbar'
import type {
    PermissionScheme,
    CreatePermissionSchemeRequest,
    UpdatePermissionSchemeRequest,
} from '@/types/schemas'

export function usePermissionSchemes() {
    const { showSuccess, showError } = useSnackbar()

    const loading = ref(false)
    const error = ref('')
    const schemes = ref<PermissionScheme[]>([])

    const loadSchemes = async (page = 1, perPage = 20) => {
        loading.value = true
        error.value = ''

        try {
            const response = await typedHttpClient.getPermissionSchemes(page, perPage)
            schemes.value = response.data
            return response
        } catch (err) {
            console.error('Failed to load permission schemes:', err)
            const errorMessage =
                err instanceof Error ? err.message : 'Failed to load permission schemes'
            error.value = errorMessage
            showError(errorMessage)
            throw err
        } finally {
            loading.value = false
        }
    }

    const createScheme = async (data: CreatePermissionSchemeRequest) => {
        try {
            await typedHttpClient.createPermissionScheme(data)
            showSuccess('Permission scheme created successfully')
        } catch (err) {
            const message =
                err instanceof Error ? err.message : 'Failed to create permission scheme'
            showError(message)
            throw err
        }
    }

    const updateScheme = async (uuid: string, data: UpdatePermissionSchemeRequest) => {
        try {
            await typedHttpClient.updatePermissionScheme(uuid, data)
            showSuccess('Permission scheme updated successfully')
        } catch (err) {
            const message =
                err instanceof Error ? err.message : 'Failed to update permission scheme'
            showError(message)
            throw err
        }
    }

    const deleteScheme = async (uuid: string) => {
        try {
            await typedHttpClient.deletePermissionScheme(uuid)
            showSuccess('Permission scheme deleted successfully')
        } catch (err) {
            const message =
                err instanceof Error ? err.message : 'Failed to delete permission scheme'
            showError(message)
            throw err
        }
    }

    return {
        loading,
        error,
        schemes,
        loadSchemes,
        createScheme,
        updateScheme,
        deleteScheme,
    }
}
