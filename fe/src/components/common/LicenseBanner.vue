<template>
    <DismissableBanner
        :show="licenseStore.shouldShowBanner"
        :message="bannerMessage"
        :dismiss-label="t('license.banner.dismiss')"
        type="error"
        icon="alert-circle"
        @dismiss="handleDismiss"
    />
</template>

<script setup lang="ts">
    import { computed } from 'vue'
    import { useLicenseStore } from '@/stores/license'
    import { useTranslations } from '@/composables/useTranslations'
    import DismissableBanner from '@/components/common/DismissableBanner.vue'

    const licenseStore = useLicenseStore()
    const { t } = useTranslations()

    const bannerMessage = computed(() => {
        const state = licenseStore.licenseStatus?.state
        if (!state) {
            return t('license.banner.no_license')
        }

        switch (state) {
            case 'none':
                return t('license.banner.no_license')
            case 'invalid':
                return t('license.banner.invalid_license')
            case 'error':
                return t('license.banner.error_license')
            default:
                return t('license.banner.no_license')
        }
    })

    const handleDismiss = (): void => {
        licenseStore.dismissLicenseBanner()
    }
</script>
