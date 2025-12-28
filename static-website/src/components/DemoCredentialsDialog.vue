<template>
    <v-dialog
        v-model="model"
        max-width="440"
    >
        <v-card>
            <v-card-title>{{ title }}</v-card-title>
            <v-card-text>
                <p class="hint">{{ hint }}</p>
                <v-alert
                    v-if="isMobile"
                    type="info"
                    variant="tonal"
                    prominent
                    class="mb-4 mobile-warning"
                >
                    <template #prepend>
                        <v-icon>info</v-icon>
                    </template>
                    <div class="mobile-warning-text">{{ mobileWarning }}</div>
                </v-alert>
                <div class="credentials">
                    <div class="row">
                        <span>{{ usernameLabel }}</span>
                        <code>{{ username }}</code>
                    </div>
                    <div class="row">
                        <span>{{ passwordLabel }}</span>
                        <code>{{ password }}</code>
                    </div>
                </div>
            </v-card-text>
            <v-card-actions class="actions">
                <v-btn
                    variant="text"
                    @click="close"
                    >{{ cancelLabel }}</v-btn
                >
                <v-btn
                    color="primary"
                    @click="openDemo"
                    >{{ openDemoLabel }}</v-btn
                >
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>

<script setup lang="ts">
    import { computed, ref, onMounted, onUnmounted } from 'vue'
    import { env } from '@/env-check'
    import { useTranslations } from '@/composables/useTranslations'

    const props = defineProps<{
        modelValue: boolean
        title: string
        hint: string
        usernameLabel: string
        passwordLabel: string
        cancelLabel: string
        openDemoLabel: string
    }>()

    const { t } = useTranslations()
    const isMobile = ref(false)

    const mobileWarning = computed(() => t('cta.demo_overlay.mobile_warning'))

    const updateIsMobile = () => {
        isMobile.value = window.innerWidth < 1200
    }

    onMounted(() => {
        updateIsMobile()
        window.addEventListener('resize', updateIsMobile)
    })

    onUnmounted(() => {
        window.removeEventListener('resize', updateIsMobile)
    })

    const emit = defineEmits<{
        (e: 'update:modelValue', value: boolean): void
        (e: 'open-demo'): void
    }>()

    const username = 'admin'
    const password = 'adminadmin'
    const model = computed({
        get: () => props.modelValue,
        set: value => emit('update:modelValue', value),
    })

    const openDemo = () => {
        emit('open-demo')
        if (env.demoUrl) {
            window.open(env.demoUrl, '_blank', 'noopener')
        }
        emit('update:modelValue', false)
    }

    const close = () => emit('update:modelValue', false)
</script>

<style scoped>
    .hint {
        margin: 0 0 12px;
        color: rgb(var(--v-theme-on-surface-variant));
    }

    .credentials {
        display: grid;
        gap: 8px;
    }

    .row {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 8px 12px;
        border-radius: 8px;
        background: rgba(var(--v-theme-primary), 0.06);
        font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace;
    }

    code {
        background: rgba(var(--v-theme-surface-variant, 0, 0, 0), 0.06);
        padding: 2px 6px;
        border-radius: 6px;
    }

    .actions {
        justify-content: flex-end;
        gap: 8px;
    }

    .mobile-warning {
        padding: 12px 16px;
    }

    .mobile-warning-text {
        padding: 4px 0;
        line-height: 1.5;
        word-wrap: break-word;
    }
</style>
