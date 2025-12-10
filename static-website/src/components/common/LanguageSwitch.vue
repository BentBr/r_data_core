<template>
    <v-menu offset-y>
        <template #activator="{ props }">
            <v-btn
                icon
                size="small"
                v-bind="props"
                :title="t('general.language.switch')"
            >
                <component
                    :is="getCurrentFlagComponent()"
                    :width="20"
                    :height="15"
                />
            </v-btn>
        </template>
        <v-list min-width="140">
            <v-list-item
                v-for="lang in availableLanguages"
                :key="lang"
                :class="{ 'v-list-item--active': currentLanguage === lang }"
                class="px-4 py-2"
                @click="handleLanguageChange(lang)"
            >
                <template #prepend>
                    <div class="flag-wrapper">
                        <component
                            :is="getFlagComponent(lang)"
                            :width="24"
                            :height="18"
                        />
                    </div>
                </template>
                <v-list-item-title>{{ lang.toUpperCase() }}</v-list-item-title>
            </v-list-item>
        </v-list>
    </v-menu>
</template>

<script setup lang="ts">
    import { useTranslations } from '@/composables/useTranslations'
    import { useRouter, useRoute } from 'vue-router'
    import UkFlag from './flags/UkFlag.vue'
    import GermanFlag from './flags/GermanFlag.vue'

    const { currentLanguage, availableLanguages, setLanguage, t } = useTranslations()
    const router = useRouter()
    const route = useRoute()

    const getFlagComponent = (lang: string) => {
        const flags = {
            en: UkFlag,
            de: GermanFlag,
        }
        return flags[lang as keyof typeof flags] ?? UkFlag
    }

    const getCurrentFlagComponent = () => {
        return getFlagComponent(currentLanguage.value)
    }

    const handleLanguageChange = async (lang: string) => {
        if (lang === 'en' || lang === 'de') {
            await setLanguage(lang)

            // Update route to include language parameter
            const pathWithoutLang = route.path.replace(/^\/(en|de)/, '') || '/'
            const newPath = `/${lang}${pathWithoutLang === '/' ? '' : pathWithoutLang}`
            await router.push(newPath)
        }
    }
</script>

<style scoped>
    .v-list-item {
        cursor: pointer;
        min-height: 48px;
    }

    .v-list-item--active {
        background-color: rgba(var(--v-theme-primary), 0.08);
    }

    .flag-wrapper {
        margin-right: 8px;
    }
</style>
