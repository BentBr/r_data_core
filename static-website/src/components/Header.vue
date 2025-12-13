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
                <router-link :to="getLocalizedPath('/roadmap')">{{ t('nav.roadmap') }}</router-link>
                <router-link :to="getLocalizedPath('/use-cases')">{{
                    t('nav.use_cases')
                }}</router-link>
            </nav>
            <div class="actions">
                <ThemeToggle />
                <LanguageSwitch />
                <v-btn
                    color="primary"
                    size="small"
                    rounded
                    class="demo-btn-desktop"
                    @click="openDemo"
                >
                    {{ t('cta.primary') }}
                </v-btn>
                <v-btn
                    icon
                    variant="text"
                    class="burger-btn"
                    :aria-label="t('nav.menu', 'Menu')"
                    @click="mobileDrawer = true"
                >
                    <SmartIcon
                        icon="menu"
                        size="24"
                    />
                </v-btn>
            </div>
        </div>
    </header>

    <!-- Mobile Navigation Drawer -->
    <v-navigation-drawer
        v-model="mobileDrawer"
        location="right"
        temporary
        class="mobile-drawer"
    >
        <div class="mobile-drawer__header">
            <router-link
                :to="getLocalizedPath('/')"
                class="brand"
                @click="mobileDrawer = false"
            >
                <SmartIcon
                    icon="database"
                    size="24"
                    color="primary"
                />
                <span class="brand__name">RDataCore</span>
            </router-link>
            <v-btn
                icon
                variant="text"
                @click="mobileDrawer = false"
            >
                <SmartIcon
                    icon="x"
                    size="24"
                />
            </v-btn>
        </div>
        <v-divider />
        <v-list
            nav
            class="mobile-nav-list"
        >
            <v-list-item
                :to="getLocalizedPath('/')"
                @click="mobileDrawer = false"
            >
                <template #prepend>
                    <SmartIcon
                        icon="home"
                        size="20"
                    />
                </template>
                <v-list-item-title>{{ t('nav.home') }}</v-list-item-title>
            </v-list-item>
            <v-list-item
                :to="getLocalizedPath('/about')"
                @click="mobileDrawer = false"
            >
                <template #prepend>
                    <SmartIcon
                        icon="info"
                        size="20"
                    />
                </template>
                <v-list-item-title>{{ t('nav.about') }}</v-list-item-title>
            </v-list-item>
            <v-list-item
                :to="getLocalizedPath('/pricing')"
                @click="mobileDrawer = false"
            >
                <template #prepend>
                    <SmartIcon
                        icon="credit-card"
                        size="20"
                    />
                </template>
                <v-list-item-title>{{ t('nav.pricing') }}</v-list-item-title>
            </v-list-item>
            <v-list-item
                :to="getLocalizedPath('/roadmap')"
                @click="mobileDrawer = false"
            >
                <template #prepend>
                    <SmartIcon
                        icon="map"
                        size="20"
                    />
                </template>
                <v-list-item-title>{{ t('nav.roadmap') }}</v-list-item-title>
            </v-list-item>
            <v-list-item
                :to="getLocalizedPath('/use-cases')"
                @click="mobileDrawer = false"
            >
                <template #prepend>
                    <SmartIcon
                        icon="briefcase"
                        size="20"
                    />
                </template>
                <v-list-item-title>{{ t('nav.use_cases') }}</v-list-item-title>
            </v-list-item>
        </v-list>
        <v-divider />
        <div class="mobile-drawer__footer">
            <v-btn
                color="primary"
                size="large"
                rounded
                block
                @click="openDemoMobile"
            >
                {{ t('cta.primary') }}
            </v-btn>
        </div>
    </v-navigation-drawer>
</template>

<script setup lang="ts">
    import { ref } from 'vue'
    import LanguageSwitch from './common/LanguageSwitch.vue'
    import ThemeToggle from './common/ThemeToggle.vue'
    import SmartIcon from './common/SmartIcon.vue'
    import { useTranslations } from '@/composables/useTranslations'
    import { useRoute } from 'vue-router'

    const { t, currentLanguage } = useTranslations()
    const route = useRoute()
    const mobileDrawer = ref(false)

    const getLocalizedPath = (path: string) => {
        // Get current language from route or default to currentLanguage
        const lang = (route.params.lang as string) || currentLanguage.value
        return `/${lang}${path === '/' ? '' : path}`
    }

    const openDemo = () => {
        window.dispatchEvent(new CustomEvent('open-demo'))
    }

    const openDemoMobile = () => {
        mobileDrawer.value = false
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

    .burger-btn {
        display: flex;
    }

    .demo-btn-desktop {
        display: none;
    }

    @media (min-width: 960px) {
        .nav {
            display: flex;
        }
        .actions {
            margin-left: 0;
        }
        .burger-btn {
            display: none;
        }
        .demo-btn-desktop {
            display: inline-flex;
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

    /* Mobile Drawer Styles */
    .mobile-drawer__header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 12px 16px;
    }

    .mobile-drawer__header .brand {
        display: flex;
        align-items: center;
        gap: 8px;
        text-decoration: none;
        color: inherit;
    }

    .mobile-drawer__header .brand__name {
        font-weight: 700;
        font-size: 1.1rem;
    }

    .mobile-nav-list {
        padding: 8px;
    }

    .mobile-nav-list :deep(.v-list-item) {
        border-radius: 8px;
        margin-bottom: 4px;
    }

    .mobile-nav-list :deep(.v-list-item__prepend) {
        margin-right: 12px;
    }

    .mobile-drawer__footer {
        padding: 16px;
    }
</style>
