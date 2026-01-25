<template>
    <DismissableBanner
        :show="shouldShowBanner"
        :message="t('auth.mobile_warning')"
        :dismiss-label="t('auth.mobile_warning_dismiss')"
        type="warning"
        icon="alert-triangle"
        @dismiss="handleDismiss"
    />
</template>

<script setup lang="ts">
    import { computed, ref, onMounted, onUnmounted } from 'vue'
    import { useAuthStore } from '@/stores/auth'
    import { useTranslations } from '@/composables/useTranslations'
    import DismissableBanner from '@/components/common/DismissableBanner.vue'

    const authStore = useAuthStore()
    const { t } = useTranslations()
    const isMobile = ref(false)

    const updateIsMobile = () => {
        isMobile.value = window.innerWidth < 1200
    }

    const shouldShowBanner = computed(() => {
        // Only show on mobile (< 1200px) and if not dismissed
        return isMobile.value && !authStore.isMobileWarningDismissed
    })

    const handleDismiss = (): void => {
        authStore.dismissMobileWarningBanner()
    }

    onMounted(() => {
        updateIsMobile()
        window.addEventListener('resize', updateIsMobile)
    })

    onUnmounted(() => {
        window.removeEventListener('resize', updateIsMobile)
    })
</script>
