<template>
    <v-alert
        v-if="authStore.isDefaultPasswordInUse"
        type="warning"
        variant="tonal"
        prominent
        closable
        class="mb-4"
        @click:close="handleDismiss"
    >
        <template #prepend>
            <SmartIcon
                icon="alert-triangle"
                size="sm"
            />
        </template>
        <div class="d-flex align-center justify-space-between">
            <span>{{ t('auth.default_password_warning') }}</span>
            <v-btn
                variant="text"
                size="small"
                @click="handleDismiss"
            >
                {{ t('auth.default_password_warning_dismiss') }}
            </v-btn>
        </div>
    </v-alert>
</template>

<script setup lang="ts">
    import { useAuthStore } from '@/stores/auth'
    import { useTranslations } from '@/composables/useTranslations'
    import SmartIcon from '@/components/common/SmartIcon.vue'

    const authStore = useAuthStore()
    const { t } = useTranslations()

    const handleDismiss = (): void => {
        authStore.dismissDefaultPasswordBanner()
    }
</script>

<style scoped>
    .v-alert {
        border-left: 4px solid rgb(var(--v-theme-warning));
    }
</style>
