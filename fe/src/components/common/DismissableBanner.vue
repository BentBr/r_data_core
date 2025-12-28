<template>
    <v-alert
        v-if="show"
        :type="type"
        variant="tonal"
        prominent
        closable
        :class="['mb-4', 'dismissable-banner', `banner-type-${type}`]"
        @click:close="handleDismiss"
    >
        <template #prepend>
            <SmartIcon
                :icon="icon"
                size="sm"
            />
        </template>
        <div class="d-flex align-center justify-space-between">
            <span>{{ message }}</span>
            <v-btn
                variant="text"
                size="small"
                @click="handleDismiss"
            >
                {{ dismissLabel }}
            </v-btn>
        </div>
    </v-alert>
</template>

<script setup lang="ts">
    import SmartIcon from '@/components/common/SmartIcon.vue'

    defineProps<{
        show: boolean
        message: string
        dismissLabel: string
        type?: 'warning' | 'info' | 'error' | 'success'
        icon?: string
    }>()

    const emit = defineEmits<{
        (e: 'dismiss'): void
    }>()

    const handleDismiss = (): void => {
        emit('dismiss')
    }
</script>

<style scoped>
    .dismissable-banner {
        border-left: 4px solid rgb(var(--v-theme-warning));
    }

    .banner-type-info {
        border-left-color: rgb(var(--v-theme-info));
    }

    .banner-type-error {
        border-left-color: rgb(var(--v-theme-error));
    }

    .banner-type-success {
        border-left-color: rgb(var(--v-theme-success));
    }
</style>
