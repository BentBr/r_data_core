import { ref } from 'vue'
import { typedHttpClient } from '@/api/typed-client'
import { useErrorHandler } from '@/composables/useErrorHandler'
import { useTranslations } from '@/composables/useTranslations'
import type { UserResponse, CreateUserRequest, UpdateUserRequest } from '@/types/schemas'

export function useUsers() {
    const { handleError, handleSuccess } = useErrorHandler()
    const { t } = useTranslations()

    const loading = ref(false)
    const error = ref('')
    const users = ref<UserResponse[]>([])

    const loadUsers = async (
        page = 1,
        perPage = 20,
        sortBy?: string | null,
        sortOrder?: 'asc' | 'desc' | null
    ) => {
        loading.value = true
        error.value = ''

        try {
            const response = await typedHttpClient.getUsers(page, perPage, sortBy, sortOrder)
            users.value = response.data
            return response
        } catch (err) {
            console.error('Failed to load users:', err)
            const errorMessage = err instanceof Error ? err.message : 'Failed to load users'
            error.value = errorMessage
            handleError(err)
            throw err
        } finally {
            loading.value = false
        }
    }

    const createUser = async (data: CreateUserRequest) => {
        try {
            await typedHttpClient.createUser(data)
            handleSuccess(t('users.create.success'))
        } catch (err) {
            handleError(err)
            throw err
        }
    }

    const updateUser = async (uuid: string, data: UpdateUserRequest) => {
        try {
            await typedHttpClient.updateUser(uuid, data)
            handleSuccess(t('users.update.success'))
        } catch (err) {
            handleError(err)
            throw err
        }
    }

    const deleteUser = async (uuid: string) => {
        try {
            await typedHttpClient.deleteUser(uuid)
            handleSuccess(t('users.delete.success'))
        } catch (err) {
            handleError(err)
            throw err
        }
    }

    const getUser = async (uuid: string) => {
        loading.value = true
        error.value = ''

        try {
            return await typedHttpClient.getUser(uuid)
        } catch (err) {
            console.error('Failed to load user:', err)
            const errorMessage = err instanceof Error ? err.message : 'Failed to load user'
            error.value = errorMessage
            handleError(err)
            throw err
        } finally {
            loading.value = false
        }
    }

    const getUserRoles = async (uuid: string) => {
        try {
            return await typedHttpClient.getUserRoles(uuid)
        } catch (err) {
            handleError(err)
            throw err
        }
    }

    const assignRolesToUser = async (uuid: string, roleUuids: string[]) => {
        try {
            await typedHttpClient.assignRolesToUser(uuid, roleUuids)
            handleSuccess(t('users.roles.assign.success'))
        } catch (err) {
            handleError(err)
            throw err
        }
    }

    return {
        loading,
        error,
        users,
        loadUsers,
        createUser,
        updateUser,
        deleteUser,
        getUser,
        getUserRoles,
        assignRolesToUser,
    }
}
