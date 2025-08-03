import { ref } from 'vue'
import type { SnackbarConfig } from '@/types/schemas'

export function useSnackbar() {
    const currentSnackbar = ref<SnackbarConfig | null>(null)

    const showSuccess = (message: string, timeout = 3000) => {
        currentSnackbar.value = {
            message,
            color: 'success',
            timeout,
            persistent: false,
        }
    }

    const showError = (message: string, timeout = 5000) => {
        currentSnackbar.value = {
            message,
            color: 'error',
            timeout,
            persistent: false,
        }
    }

    const showWarning = (message: string, timeout = 4000) => {
        currentSnackbar.value = {
            message,
            color: 'warning',
            timeout,
            persistent: false,
        }
    }

    const showInfo = (message: string, timeout = 3000) => {
        currentSnackbar.value = {
            message,
            color: 'info',
            timeout,
            persistent: false,
        }
    }

    const clearSnackbar = () => {
        currentSnackbar.value = null
    }

    return {
        currentSnackbar,
        showSuccess,
        showError,
        showWarning,
        showInfo,
        clearSnackbar,
    }
}
