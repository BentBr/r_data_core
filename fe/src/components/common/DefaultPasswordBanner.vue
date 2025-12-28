<template>
    <DismissableBanner
        :show="authStore.isDefaultPasswordInUse"
        :message="t('auth.default_password_warning')"
        :dismiss-label="t('auth.default_password_warning_dismiss')"
        type="warning"
        icon="alert-triangle"
        @dismiss="handleDismiss"
    />
</template>

<script setup lang="ts">
    import { useAuthStore } from '@/stores/auth'
    import { useTranslations } from '@/composables/useTranslations'
    import DismissableBanner from '@/components/common/DismissableBanner.vue'

    const authStore = useAuthStore()
    const { t } = useTranslations()

    const handleDismiss = (): void => {
        authStore.dismissDefaultPasswordBanner()
    }
</script>
