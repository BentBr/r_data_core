<template>
    <v-menu offset-y>
        <template v-slot:activator="{ props }">
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
        <v-list min-width="120">
            <v-list-item
                v-for="lang in availableLanguages"
                :key="lang"
                :class="{ 'v-list-item--active': currentLanguage === lang }"
                class="px-4 py-2"
                @click="setLanguage(lang)"
            >
                <template v-slot:prepend>
                    <component
                        :is="getFlagComponent(lang)"
                        :width="24"
                        :height="18"
                    />
                </template>
            </v-list-item>
        </v-list>
    </v-menu>
</template>

<script setup lang="ts">
    import { useTranslations } from '@/composables/useTranslations'
    import UkFlag from './flags/UkFlag.vue'
    import GermanFlag from './flags/GermanFlag.vue'

    const { currentLanguage, availableLanguages, setLanguage, t } = useTranslations()

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
</script>

<style scoped>
    .v-list-item {
        cursor: pointer;
        min-height: 48px;
    }

    .v-list-item--active {
        background-color: rgba(var(--v-theme-primary), 0.1);
    }
</style>
