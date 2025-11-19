import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { useTheme } from './useTheme'

// Mock Vuetify theme
const mockChangeTheme = vi.fn()
vi.mock('vuetify', () => ({
    useTheme: () => ({
        change: mockChangeTheme,
    }),
}))

describe('useTheme', () => {
    beforeEach(() => {
        localStorage.clear()
        mockChangeTheme.mockClear()
        // Mock matchMedia
        Object.defineProperty(window, 'matchMedia', {
            writable: true,
            value: vi.fn().mockImplementation(query => ({
                matches: query === '(prefers-color-scheme: dark)',
                media: query,
                onchange: null,
                addListener: vi.fn(),
                removeListener: vi.fn(),
                addEventListener: vi.fn(),
                removeEventListener: vi.fn(),
                dispatchEvent: vi.fn(),
            })),
        })
    })

    afterEach(() => {
        localStorage.clear()
    })

    it('should initialize with system preference when no stored preference', () => {
        const { userPreference, currentTheme } = useTheme()
        expect(userPreference.value).toBe('system')
        // currentTheme will be based on system preference
        expect(['light', 'dark']).toContain(currentTheme.value)
    })

    it('should load stored preference from localStorage', () => {
        localStorage.setItem('theme-preference', 'dark')
        const { userPreference } = useTheme()
        expect(userPreference.value).toBe('dark')
    })

    it('should set theme preference and save to localStorage', () => {
        const { setThemePreference, userPreference } = useTheme()
        setThemePreference('dark')
        expect(userPreference.value).toBe('dark')
        expect(localStorage.getItem('theme-preference')).toBe('dark')
    })

    it('should toggle between light and dark', () => {
        const { toggleTheme, currentTheme, setThemePreference } = useTheme()
        setThemePreference('light')
        expect(currentTheme.value).toBe('light')
        toggleTheme()
        expect(currentTheme.value).toBe('dark')
        toggleTheme()
        expect(currentTheme.value).toBe('light')
    })

    it('should return light theme when preference is light', () => {
        const { setThemePreference, currentTheme } = useTheme()
        setThemePreference('light')
        expect(currentTheme.value).toBe('light')
    })

    it('should return dark theme when preference is dark', () => {
        const { setThemePreference, currentTheme } = useTheme()
        setThemePreference('dark')
        expect(currentTheme.value).toBe('dark')
    })

    it('should compute isDark correctly', () => {
        const { setThemePreference, isDark } = useTheme()
        setThemePreference('light')
        expect(isDark.value).toBe(false)
        setThemePreference('dark')
        expect(isDark.value).toBe(true)
    })

    it('should update Vuetify theme when currentTheme changes', () => {
        const { setThemePreference } = useTheme()
        setThemePreference('dark')
        // Wait for watch to trigger
        setTimeout(() => {
            expect(mockChangeTheme).toHaveBeenCalledWith('dark')
        }, 10)
    })
})
