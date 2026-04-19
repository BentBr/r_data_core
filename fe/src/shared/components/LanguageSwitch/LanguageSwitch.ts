import { defineComponent } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import UkFlag from '../flags/UkFlag/index.vue'
import GermanFlag from '../flags/GermanFlag/index.vue'

export default defineComponent({
    name: 'LanguageSwitch',
    components: {
        UkFlag,
        GermanFlag,
    },
    setup() {
        const { currentLanguage, availableLanguages, setLanguage, t } = useTranslations()

        const getFlagComponent = (lang: string) => {
            const flags = {
                en: UkFlag,
                de: GermanFlag,
            }
            return flags[lang as keyof typeof flags] ?? UkFlag
        }

        const getCurrentFlagComponent = () => getFlagComponent(currentLanguage.value)

        return {
            currentLanguage, availableLanguages, setLanguage, t,
            getFlagComponent, getCurrentFlagComponent,
        }
    },
})
