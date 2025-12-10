<template>
    <header class="header">
        <div class="header__inner">
            <router-link
                :to="getLocalizedPath('/')"
                class="brand"
            >
                <SmartIcon
                    icon="database"
                    size="24"
                    color="primary"
                />
                <div class="brand__text">
                    <span class="brand__name">RDataCore</span>
                    <span class="brand__subline">by Slothlike</span>
                </div>
            </router-link>
            <nav class="nav">
                <router-link :to="getLocalizedPath('/')">{{ t('nav.home') }}</router-link>
                <router-link :to="getLocalizedPath('/about')">{{ t('nav.about') }}</router-link>
                <router-link :to="getLocalizedPath('/pricing')">{{ t('nav.pricing') }}</router-link>
            </nav>
            <div class="actions">
                <ThemeToggle />
                <LanguageSwitch />
                <v-btn
                    color="primary"
                    size="small"
                    rounded
                    @click="openDemo"
                >
                    {{ t('cta.primary') }}
                </v-btn>
            </div>
        </div>
    </header>
</template>

<script setup lang="ts">
    import LanguageSwitch from './common/LanguageSwitch.vue'
    import ThemeToggle from './common/ThemeToggle.vue'
    import SmartIcon from './common/SmartIcon.vue'
    import { useTranslations } from '@/composables/useTranslations'
    import { useRoute } from 'vue-router'

    const { t, currentLanguage } = useTranslations()
    const route = useRoute()

    const getLocalizedPath = (path: string) => {
        // Get current language from route or default to currentLanguage
        const lang = (route.params.lang as string) || currentLanguage.value
        return `/${lang}${path === '/' ? '' : path}`
    }

    const openDemo = () => {
        window.dispatchEvent(new CustomEvent('open-demo'))
    }
</script>

<style scoped>
    .header {
        position: fixed;
        top: 0;
        left: 0;
        right: 0;
        z-index: 1000;
        background: rgba(var(--v-theme-surface), 0.7);
        backdrop-filter: blur(12px) saturate(180%);
        -webkit-backdrop-filter: blur(12px) saturate(180%);
        border-bottom: 1px solid rgba(var(--v-theme-on-surface), 0.08);
    }

    .header__inner {
        max-width: 1200px;
        margin: 0 auto;
        padding: 12px 24px;
        display: flex;
        align-items: center;
        gap: 24px;
    }

    .brand {
        display: inline-flex;
        align-items: center;
        gap: 10px;
        text-decoration: none;
        color: inherit;
        transition: opacity 0.2s ease;
    }

    .brand:hover {
        opacity: 0.8;
    }

    .brand__text {
        display: flex;
        flex-direction: column;
        gap: 2px;
    }

    .brand__name {
        font-weight: 700;
        font-size: 1.1rem;
        line-height: 1.2;
        color: rgb(var(--v-theme-on-surface));
    }

    .brand__subline {
        font-size: 0.65rem;
        font-weight: 400;
        color: rgb(var(--v-theme-on-surface-variant));
        line-height: 1;
    }

    .nav {
        display: none;
        gap: 24px;
        margin-left: auto;
    }

    .nav a {
        color: rgb(var(--v-theme-on-surface));
        text-decoration: none;
        font-weight: 500;
        font-size: 0.95rem;
        transition: color 0.2s ease;
        position: relative;
    }

    .nav a:hover {
        color: rgb(var(--v-theme-primary));
    }

    .nav a.router-link-active::after {
        content: '';
        position: absolute;
        bottom: -8px;
        left: 0;
        right: 0;
        height: 2px;
        background: rgb(var(--v-theme-primary));
        border-radius: 2px;
    }

    .actions {
        display: flex;
        gap: 8px;
        margin-left: auto;
        align-items: center;
    }

    @media (min-width: 960px) {
        .nav {
            display: flex;
        }
        .actions {
            margin-left: 0;
        }
    }

    @media (max-width: 959px) {
        .header__inner {
            padding: 12px 16px;
        }

        .brand__text {
            display: none;
        }
    }
</style>
