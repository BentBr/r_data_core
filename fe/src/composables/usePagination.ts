import { ref, watch } from 'vue'

interface PaginationState {
    page: number
    itemsPerPage: number
}

export function usePagination(key: string, defaultItemsPerPage = 10) {
    // Load initial state from localStorage
    const loadState = (): PaginationState => {
        try {
            const stored = localStorage.getItem(`pagination_${key}`)
            if (stored) {
                const state = JSON.parse(stored)
                return {
                    page: state.page || 1,
                    itemsPerPage: state.itemsPerPage || defaultItemsPerPage,
                }
            }
        } catch (error) {
            console.warn('Failed to load pagination state from localStorage:', error)
        }
        return {
            page: 1,
            itemsPerPage: defaultItemsPerPage,
        }
    }

    // Initialize state
    const state = ref<PaginationState>(loadState())

    // Save state to localStorage
    const saveState = (newState: PaginationState) => {
        try {
            localStorage.setItem(`pagination_${key}`, JSON.stringify(newState))
        } catch (error) {
            console.warn('Failed to save pagination state to localStorage:', error)
        }
    }

    // Watch for changes and save to localStorage
    watch(
        state,
        newState => {
            saveState(newState)
        },
        { deep: true }
    )

    // Helper functions
    const setPage = (page: number) => {
        state.value.page = page
    }

    const setItemsPerPage = (itemsPerPage: number) => {
        state.value.itemsPerPage = itemsPerPage
        // Reset to first page when changing items per page
        state.value.page = 1
    }

    const reset = () => {
        state.value = {
            page: 1,
            itemsPerPage: defaultItemsPerPage,
        }
    }

    return {
        state: state.value,
        setPage,
        setItemsPerPage,
        reset,
    }
}
