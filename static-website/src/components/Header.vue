<template>
    <header class="header">
        <div class="header__inner">
            <div class="brand">
                <SmartIcon
                    icon="database"
                    size="24"
                />
                <span>RDataCore</span>
            </div>
            <nav class="nav">
                <a href="#features">{{ t('nav.features') }}</a>
                <a href="#benefits">{{ t('nav.benefits') }}</a>
                <a href="#pricing">{{ t('nav.pricing') }}</a>
            </nav>
            <div class="actions">
                <LanguageSwitch />
                <v-btn
                    color="primary"
                    size="small"
                    @click="openDemo"
                >
                    {{ t('cta.try_demo') }}
                </v-btn>
            </div>
        </div>
    </header>
</template>

<script setup lang="ts">
    import { useTranslations } from '@/composables/useTranslations'
    import LanguageSwitch from './common/LanguageSwitch.vue'
    import SmartIcon from './common/SmartIcon.vue'

    const { t } = useTranslations()

    const openDemo = () => {
        window.dispatchEvent(new CustomEvent('open-demo'))
    }
</script>

<style scoped>
    .header {
        position: sticky;
        top: 0;
        z-index: 10;
        background: rgba(var(--v-theme-surface), 0.95);
        backdrop-filter: blur(8px);
        border-bottom: 1px solid rgba(var(--v-theme-border), 0.4);
    }

    .header__inner {
        max-width: 1200px;
        margin: 0 auto;
        padding: 14px 16px;
        display: flex;
        align-items: center;
        gap: 16px;
    }

    .brand {
        display: inline-flex;
        align-items: center;
        gap: 8px;
        font-weight: 700;
        font-size: 1.05rem;
    }

    .nav {
        display: none;
        gap: 16px;
        margin-left: auto;
    }

    .nav a {
        color: rgb(var(--v-theme-on-background));
        text-decoration: none;
        font-weight: 500;
    }

    .nav a:hover {
        color: rgb(var(--v-theme-primary));
    }

    .actions {
        display: flex;
        gap: 12px;
        margin-left: auto;
        align-items: center;
    }

    @media (min-width: 900px) {
        .nav {
            display: flex;
        }
        .actions {
            margin-left: 0;
        }
    }
</style>
