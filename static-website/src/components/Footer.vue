<template>
    <footer class="footer">
        <div class="footer__inner">
            <div class="brand">
                <SmartIcon
                    icon="database"
                    size="20"
                />
                <span>RDataCore</span>
            </div>
            <div class="links">
                <router-link :to="getLocalizedPath('/')">{{ t('nav.home') }}</router-link>
                <router-link :to="getLocalizedPath('/about')">{{ t('nav.about') }}</router-link>
                <router-link :to="getLocalizedPath('/pricing')">{{ t('nav.pricing') }}</router-link>
                <router-link :to="getLocalizedPath('/imprint')">{{
                    t('footer.imprint')
                }}</router-link>
                <router-link :to="getLocalizedPath('/privacy')">{{
                    t('footer.privacy')
                }}</router-link>
            </div>
            <div class="footer__bottom">
                <p class="copyright">{{ t('footer.rights') }}</p>
                <p class="made-with">{{ t('footer.made_with') }}</p>
            </div>
        </div>
    </footer>
</template>

<script setup lang="ts">
    import { useTranslations } from '@/composables/useTranslations'
    import SmartIcon from './common/SmartIcon.vue'
    import { useRoute } from 'vue-router'

    const { t, currentLanguage } = useTranslations()
    const route = useRoute()

    const getLocalizedPath = (path: string) => {
        // Get current language from route or default to currentLanguage
        const lang = (route.params.lang as string) || currentLanguage.value
        return `/${lang}${path === '/' ? '' : path}`
    }
</script>

<style scoped>
    .footer {
        background: rgb(var(--v-theme-surface-variant));
        color: rgb(var(--v-theme-on-surface));
        padding: 32px 16px;
    }

    .footer__inner {
        max-width: 1200px;
        margin: 0 auto;
        display: grid;
        gap: 16px;
    }

    .brand {
        display: inline-flex;
        align-items: center;
        gap: 8px;
        font-weight: 700;
    }

    .links {
        display: flex;
        gap: 16px;
        flex-wrap: wrap;
    }

    .links a {
        color: inherit;
        text-decoration: none;
        font-weight: 500;
    }

    .links a:hover {
        color: rgb(var(--v-theme-primary));
    }

    .footer__bottom {
        display: flex;
        flex-direction: column;
        gap: 8px;
        padding-top: 16px;
        border-top: 1px solid rgba(var(--v-theme-on-surface-variant), 0.2);
    }

    .copyright,
    .made-with {
        margin: 0;
        color: rgb(var(--v-theme-on-surface-variant));
        font-size: 0.875rem;
    }

    @media (min-width: 600px) {
        .footer__bottom {
            flex-direction: row;
            justify-content: space-between;
            align-items: center;
        }
    }
</style>
