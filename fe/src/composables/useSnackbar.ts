import { ref } from 'vue'
import type { SnackbarConfig } from '@/types/schemas'

// Global shared state for snackbar - singleton pattern
// This ensures all useSnackbar() calls share the same snackbar state
const currentSnackbar = ref<SnackbarConfig | null>(null)

// Track the auto-clear timeout
let autoClearTimeout: ReturnType<typeof setTimeout> | null = null

// Helper to show snackbar and auto-clear after timeout
function showSnackbar(config: SnackbarConfig) {
    // Clear any existing timeout
    if (autoClearTimeout) {
        clearTimeout(autoClearTimeout)
        autoClearTimeout = null
    }

    // Set the new snackbar
    currentSnackbar.value = config

    // Auto-clear the global state after the timeout
    // This prevents the snackbar from re-appearing on navigation
    const timeout = config.timeout ?? 3000
    autoClearTimeout = setTimeout(() => {
        currentSnackbar.value = null
        autoClearTimeout = null
    }, timeout + 500) // Add 500ms buffer for animation
}

export function useSnackbar() {
    const showSuccess = (message: string, timeout = 3000) => {
        showSnackbar({
            message,
            color: 'success',
            timeout,
            persistent: false,
        })
    }

    const showError = (message: string, timeout = 5000) => {
        showSnackbar({
            message,
            color: 'error',
            timeout,
            persistent: false,
        })
    }

    const showWarning = (message: string, timeout = 4000) => {
        showSnackbar({
            message,
            color: 'warning',
            timeout,
            persistent: false,
        })
    }

    const showInfo = (message: string, timeout = 3000) => {
        showSnackbar({
            message,
            color: 'info',
            timeout,
            persistent: false,
        })
    }

    const clearSnackbar = () => {
        if (autoClearTimeout) {
            clearTimeout(autoClearTimeout)
            autoClearTimeout = null
        }
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
