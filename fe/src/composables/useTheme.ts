import { ref, computed, watch, onMounted } from 'vue'
import { useTheme as useVuetifyTheme } from 'vuetify'

type ThemeMode = 'light' | 'dark' | 'system'

export function useTheme() {
    const vuetifyTheme = useVuetifyTheme()

    // User's theme preference (system, light, dark)
    const userPreference = ref<ThemeMode>(
        (localStorage.getItem('theme-preference') as ThemeMode) || 'system'
    )

    // System preference detection
    const systemPrefersDark = ref(false)

    // Current computed theme
    const currentTheme = computed<'light' | 'dark'>(() => {
        switch (userPreference.value) {
            case 'system':
                return systemPrefersDark.value ? 'dark' : 'light'
            case 'light':
                return 'light'
            case 'dark':
                return 'dark'
            default:
                return 'light'
        }
    })

    // Is current theme dark
    const isDark = computed(() => currentTheme.value === 'dark')

    // Detect system preference
    const detectSystemPreference = () => {
        if (typeof window !== 'undefined' && window.matchMedia) {
            systemPrefersDark.value = window.matchMedia('(prefers-color-scheme: dark)').matches
        }
    }

    // Update Vuetify theme
    const updateVuetifyTheme = () => {
        vuetifyTheme.change(currentTheme.value)
    }

    // Set theme preference
    const setThemePreference = (preference: ThemeMode) => {
        userPreference.value = preference
        localStorage.setItem('theme-preference', preference)
    }

    // Toggle between light/dark (ignoring system)
    const toggleTheme = () => {
        const newTheme = currentTheme.value === 'light' ? 'dark' : 'light'
        setThemePreference(newTheme)
    }

    // Initialize system preference listener
    const initializeSystemListener = () => {
        if (typeof window !== 'undefined' && window.matchMedia) {
            const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')

            // Initial detection
            detectSystemPreference()

            // Listen for changes
            mediaQuery.addEventListener('change', e => {
                systemPrefersDark.value = e.matches
            })
        }
    }

    // Watch for theme changes and update Vuetify
    watch(currentTheme, updateVuetifyTheme, { immediate: true })

    // Initialize on mount
    onMounted(() => {
        initializeSystemListener()
    })

    return {
        userPreference,
        currentTheme,
        systemPrefersDark,
        isDark,
        setThemePreference,
        toggleTheme,
    }
}
