import { ref } from 'vue'
import { typedHttpClient } from '@/api/typed-client'
import { useSnackbar } from '@/composables/useSnackbar'
import type { UserResponse, CreateUserRequest, UpdateUserRequest } from '@/types/schemas'

export function useUsers() {
    const { showSuccess, showError } = useSnackbar()

    const loading = ref(false)
    const error = ref('')
    const users = ref<UserResponse[]>([])

    const loadUsers = async (page = 1, perPage = 20) => {
        loading.value = true
        error.value = ''

        try {
            const response = await typedHttpClient.getUsers(page, perPage)
            users.value = response.data
            return response
        } catch (err) {
            console.error('Failed to load users:', err)
            const errorMessage = err instanceof Error ? err.message : 'Failed to load users'
            error.value = errorMessage
            showError(errorMessage)
            throw err
        } finally {
            loading.value = false
        }
    }

    const createUser = async (data: CreateUserRequest) => {
        try {
            await typedHttpClient.createUser(data)
            showSuccess('User created successfully')
        } catch (err) {
            const message = err instanceof Error ? err.message : 'Failed to create user'
            showError(message)
            throw err
        }
    }

    const updateUser = async (uuid: string, data: UpdateUserRequest) => {
        try {
            await typedHttpClient.updateUser(uuid, data)
            showSuccess('User updated successfully')
        } catch (err) {
            const message = err instanceof Error ? err.message : 'Failed to update user'
            showError(message)
            throw err
        }
    }

    const deleteUser = async (uuid: string) => {
        try {
            await typedHttpClient.deleteUser(uuid)
            showSuccess('User deleted successfully')
        } catch (err) {
            const message = err instanceof Error ? err.message : 'Failed to delete user'
            showError(message)
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
            showError(errorMessage)
            throw err
        } finally {
            loading.value = false
        }
    }

    const getUserRoles = async (uuid: string) => {
        try {
            return await typedHttpClient.getUserRoles(uuid)
        } catch (err) {
            const message = err instanceof Error ? err.message : 'Failed to load user roles'
            showError(message)
            throw err
        }
    }

    const assignRolesToUser = async (uuid: string, roleUuids: string[]) => {
        try {
            await typedHttpClient.assignRolesToUser(uuid, roleUuids)
            showSuccess('Roles assigned successfully')
        } catch (err) {
            const message = err instanceof Error ? err.message : 'Failed to assign roles'
            showError(message)
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
