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
                <a href="#features">{{ t('nav.features') }}</a>
                <a href="#benefits">{{ t('nav.benefits') }}</a>
                <a href="#pricing">{{ t('nav.pricing') }}</a>
            </div>
            <p class="copyright">
                © {{ new Date().getFullYear() }} RDataCore. {{ t('footer.rights') }}
            </p>
        </div>
    </footer>
</template>

<script setup lang="ts">
    import { useTranslations } from '@/composables/useTranslations'
    import SmartIcon from './common/SmartIcon.vue'

    const { t } = useTranslations()
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

    .copyright {
        margin: 0;
        color: rgb(var(--v-theme-on-surface-variant));
    }
</style>
