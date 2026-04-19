import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { usePagination } from './usePagination'

describe('usePagination', () => {
    beforeEach(() => {
        localStorage.clear()
        vi.clearAllMocks()
    })

    afterEach(() => {
        localStorage.clear()
    })

    it('should initialize with default values when no stored state', () => {
        const pagination = usePagination('test')
        expect(pagination.state.page).toBe(1)
        expect(pagination.state.itemsPerPage).toBe(10)
    })

    it('should initialize with custom default items per page', () => {
        const pagination = usePagination('test', 20)
        expect(pagination.state.itemsPerPage).toBe(20)
    })

    it('should load state from localStorage', () => {
        localStorage.setItem('pagination_test', JSON.stringify({ page: 3, itemsPerPage: 25 }))
        const pagination = usePagination('test')
        expect(pagination.state.page).toBe(3)
        expect(pagination.state.itemsPerPage).toBe(25)
    })

    it('should handle invalid localStorage data gracefully', () => {
        localStorage.setItem('pagination_test', 'invalid json')
        const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
        const pagination = usePagination('test')
        expect(pagination.state.page).toBe(1)
        expect(pagination.state.itemsPerPage).toBe(10)
        consoleSpy.mockRestore()
    })

    it('should save state to localStorage when page changes', async () => {
        const pagination = usePagination('test')
        pagination.setPage(5)
        // Wait for watch to trigger
        await new Promise(resolve => setTimeout(resolve, 10))
        const stored = localStorage.getItem('pagination_test')
        expect(stored).toBeTruthy()
        if (stored) {
            const state = JSON.parse(stored)
            expect(state.page).toBe(5)
        }
    })

    it('should save state to localStorage when itemsPerPage changes', async () => {
        const pagination = usePagination('test')
        pagination.setItemsPerPage(50)
        // Wait for watch to trigger
        await new Promise(resolve => setTimeout(resolve, 10))
        const stored = localStorage.getItem('pagination_test')
        expect(stored).toBeTruthy()
        if (stored) {
            const state = JSON.parse(stored)
            expect(state.itemsPerPage).toBe(50)
            expect(state.page).toBe(1) // Should reset to page 1
        }
    })

    it('should reset to page 1 when changing itemsPerPage', () => {
        const pagination = usePagination('test')
        pagination.setPage(5)
        expect(pagination.state.page).toBe(5)
        pagination.setItemsPerPage(25)
        expect(pagination.state.page).toBe(1)
    })

    it('should reset to default values', async () => {
        const pagination = usePagination('test', 20)
        pagination.setPage(5)
        pagination.setItemsPerPage(30)
        await new Promise(resolve => setTimeout(resolve, 10))
        pagination.reset()
        await new Promise(resolve => setTimeout(resolve, 10))
        const stored = localStorage.getItem('pagination_test')
        expect(stored).toBeTruthy()
        if (stored) {
            const state = JSON.parse(stored)
            expect(state.page).toBe(1)
            expect(state.itemsPerPage).toBe(20)
        }
    })

    it('should use different keys for different instances', () => {
        const pagination1 = usePagination('key1')
        const pagination2 = usePagination('key2')
        pagination1.setPage(3)
        pagination2.setPage(7)
        expect(pagination1.state.page).toBe(3)
        expect(pagination2.state.page).toBe(7)
    })

    it('should handle missing page in stored state', () => {
        localStorage.setItem('pagination_test', JSON.stringify({ itemsPerPage: 25 }))
        const pagination = usePagination('test')
        expect(pagination.state.page).toBe(1)
        expect(pagination.state.itemsPerPage).toBe(25)
    })

    it('should handle missing itemsPerPage in stored state', () => {
        localStorage.setItem('pagination_test', JSON.stringify({ page: 3 }))
        const pagination = usePagination('test', 15)
        expect(pagination.state.page).toBe(3)
        expect(pagination.state.itemsPerPage).toBe(15)
    })
})
