import { ref, computed, reactive } from 'vue'

interface TranslationData {
    [key: string]: string | TranslationData
}

const AVAILABLE_LANGUAGES = ['en', 'de'] as const
type Language = (typeof AVAILABLE_LANGUAGES)[number]

const currentLanguage = ref<Language>(
    (localStorage.getItem('preferred-language') as Language) || 'en'
)
const translations = reactive<Record<Language, TranslationData>>({
    en: {},
    de: {},
})

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

async function loadTranslation(language: Language): Promise<void> {
    try {
        const translationModule = await import(`../../translations/${language}.json`)
        translations[language] = translationModule.default
    } catch (error) {
        console.warn(`Failed to load translation for language: ${language}`, error)
        translations[language] = {}
    }
}

export function useTranslations() {
    const initTranslations = async () => {
        if (Object.keys(translations.en).length === 0) {
            await Promise.all(AVAILABLE_LANGUAGES.map(lang => loadTranslation(lang)))
        }
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

    const setLanguage = async (language: Language) => {
        currentLanguage.value = language
        localStorage.setItem('preferred-language', language)
        await loadTranslation(language)
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

// Initialize translations on module load
const translationInstance = useTranslations()
void translationInstance.initTranslations()
