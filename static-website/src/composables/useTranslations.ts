import { ref, computed, reactive } from 'vue'
// Import translations synchronously to ensure they're available for SSR/SEO
import enTranslations from '../../translations/en.json'
import deTranslations from '../../translations/de.json'

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type TranslationValue = string | string[] | Record<string, any>[] | TranslationData
interface TranslationData {
    [key: string]: TranslationValue
}

const AVAILABLE_LANGUAGES = ['en', 'de'] as const
type Language = (typeof AVAILABLE_LANGUAGES)[number]

const currentLanguage = ref<Language>(
    (typeof localStorage !== 'undefined'
        ? (localStorage.getItem('preferred-language') as Language)
        : null) ?? 'en'
)

// Initialize translations synchronously - they're bundled with the app
const translations = reactive<Record<Language, TranslationData>>({
    en: enTranslations as TranslationData,
    de: deTranslations as TranslationData,
})

function getNestedProperty(obj: TranslationData, path: string): string | undefined {
    const result = path.split('.').reduce<TranslationValue | undefined>(
        (current, key) => {
            if (current && typeof current === 'object') {
                // Handle array access with numeric index
                if (Array.isArray(current)) {
                    const index = parseInt(key, 10)
                    if (!isNaN(index) && index >= 0 && index < current.length) {
                        return current[index] as TranslationValue
                    }
                    return undefined
                }
                // Handle object access
                if (key in current) {
                    return (current as TranslationData)[key]
                }
            }
            return undefined
        },
        obj as TranslationValue | undefined
    )
    return typeof result === 'string' ? result : undefined
}

// Translations are loaded synchronously at module initialization
// This function is kept for API compatibility but is now a no-op
function setTranslationLanguage(_language: Language): void {
    // No-op: translations are already loaded synchronously
}

export function useTranslations() {
    // Kept for API compatibility - translations are loaded synchronously at import
    const initTranslations = () => {
        // No-op: translations are bundled and available immediately
    }

    const currentTranslations = computed(() => {
        return translations[currentLanguage.value] || {}
    })

    const fallbackTranslations = computed(() => {
        return translations.en || {}
    })

    const t = (
        key: string,
        params?: Record<string, string> | string,
        fallback?: string
    ): string => {
        let actualParams: Record<string, string> | undefined
        let actualFallback: string | undefined

        if (typeof params === 'string') {
            actualFallback = params
        } else {
            actualParams = params
            actualFallback = fallback
        }

        let translation = getNestedProperty(currentTranslations.value, key)

        if (!translation && currentLanguage.value !== 'en') {
            translation = getNestedProperty(fallbackTranslations.value, key)
        }

        let result = translation ?? actualFallback ?? key

        if (actualParams && typeof result === 'string') {
            Object.entries(actualParams).forEach(([placeholder, value]) => {
                result = result.replace(`{${placeholder}}`, value)
            })
        }

        return result
    }

    const get = <T = unknown>(key: string): T | undefined => {
        const value = getNestedProperty(currentTranslations.value, key)
        if (value !== undefined) {
            return value as T
        }
        if (currentLanguage.value !== 'en') {
            const fallbackValue = getNestedProperty(fallbackTranslations.value, key)
            if (fallbackValue !== undefined) {
                return fallbackValue as T
            }
        }
        return undefined
    }

    const setLanguage = (language: Language) => {
        currentLanguage.value = language
        if (typeof localStorage !== 'undefined') {
            localStorage.setItem('preferred-language', language)
        }
        setTranslationLanguage(language)
    }

    const availableLanguages = computed(() => AVAILABLE_LANGUAGES)

    return {
        currentLanguage: computed(() => currentLanguage.value),
        availableLanguages,
        t,
        get,
        setLanguage,
        initTranslations,
    }
}

// Translations are now loaded synchronously at import time
// No need for async initialization
