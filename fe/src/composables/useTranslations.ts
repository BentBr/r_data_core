import { ref, computed, reactive } from 'vue'

// Translation data type
interface TranslationData {
    [key: string]: string | TranslationData
}

// Available languages
const AVAILABLE_LANGUAGES = ['en', 'de'] as const
type Language = (typeof AVAILABLE_LANGUAGES)[number]

// Current language state with localStorage persistence
const currentLanguage = ref<Language>(
    (localStorage.getItem('preferred-language') as Language | null) ?? 'en'
)
const translations = reactive<Record<Language, TranslationData>>({
    en: {},
    de: {},
})

// Helper function to get nested property by path
function getNestedProperty(obj: TranslationData, path: string): string | undefined {
    return path.split('.').reduce(
        (current, key) => {
            if (current && typeof current === 'object' && key in current) {
                const value = current[key]
                return typeof value === 'string' ? value : value
            }
            return undefined
        },
        obj as TranslationData | string | undefined
    ) as string | undefined
}

// Load translation file
async function loadTranslation(language: Language): Promise<void> {
    try {
        const translationModule = await import(`../../translations/${language}.json`)
        translations[language] = translationModule.default
    } catch (error) {
        console.warn(`Failed to load translation for language: ${language}`, error)
        // Fallback to empty object if translation file is missing
        translations[language] = {}
    }
}

// Translation composable
export function useTranslations() {
    // Initialize translations if not loaded
    const initTranslations = async () => {
        if (Object.keys(translations.en).length === 0) {
            await Promise.all(AVAILABLE_LANGUAGES.map(lang => loadTranslation(lang)))
        }
    }

    // Get current language translations
    const currentTranslations = computed(() => {
        return translations[currentLanguage.value]
    })

    // Get fallback (English) translations
    const fallbackTranslations = computed(() => {
        return translations.en
    })

    // Translate function with optional parameters
    const t = (
        key: string,
        params?: Record<string, string> | string,
        fallback?: string
    ): string => {
        // Handle the case where params is actually a fallback string
        let actualParams: Record<string, string> | undefined
        let actualFallback: string | undefined

        if (typeof params === 'string') {
            actualFallback = params
        } else {
            actualParams = params
            actualFallback = fallback
        }

        // Try current language first
        let translation = getNestedProperty(currentTranslations.value, key)

        // Fallback to English if not found
        if (!translation && currentLanguage.value !== 'en') {
            translation = getNestedProperty(fallbackTranslations.value, key)
        }

        // Return fallback text or key if no translation found
        let result = translation ?? actualFallback ?? key

        // Replace placeholders if parameters are provided
        if (actualParams && typeof result === 'string') {
            Object.entries(actualParams).forEach(([placeholder, value]) => {
                result = result.replace(`{${placeholder}}`, value)
            })
        }

        return result
    }

    // Translate error messages specifically
    const translateError = (errorMessage: string): string => {
        const lowerMessage = errorMessage.toLowerCase()

        // Map common backend error patterns to translation keys
        if (
            lowerMessage.includes('invalid credentials') ||
            lowerMessage.includes('invalid username') ||
            lowerMessage.includes('invalid password')
        ) {
            return t('auth.login.errors.invalid_credentials', errorMessage)
        }

        if (lowerMessage.includes('username') && lowerMessage.includes('required')) {
            return t('auth.login.errors.username_required', errorMessage)
        }

        if (lowerMessage.includes('password') && lowerMessage.includes('required')) {
            return t('auth.login.errors.password_required', errorMessage)
        }

        if (lowerMessage.includes('validation')) {
            return t('auth.login.errors.validation_failed', errorMessage)
        }

        if (lowerMessage.includes('network') || lowerMessage.includes('connection')) {
            return t('auth.login.errors.network_error', errorMessage)
        }

        if (lowerMessage.includes('server error') || lowerMessage.includes('internal server')) {
            return t('auth.login.errors.server_error', errorMessage)
        }

        if (lowerMessage.includes('authentication required')) {
            return t('auth.login.errors.authentication_required', errorMessage)
        }

        // Default fallback - return original message if no specific translation found
        return errorMessage
    }

    // Set language with persistence
    const setLanguage = async (language: Language) => {
        currentLanguage.value = language
        localStorage.setItem('preferred-language', language)
        await loadTranslation(language)
    }

    // Get available languages
    const availableLanguages = computed(() => AVAILABLE_LANGUAGES)

    return {
        currentLanguage: computed(() => currentLanguage.value),
        availableLanguages,
        t,
        translateError,
        setLanguage,
        initTranslations,
    }
}

// Initialize translations on module load
const translationInstance = useTranslations()
void translationInstance.initTranslations()
