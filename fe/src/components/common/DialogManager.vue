<template>
    <v-dialog
        v-model="model"
        :max-width="computedMaxWidth"
        :persistent="dialogConfig.persistent ?? false"
    >
        <v-card>
            <v-card-title class="text-h5 pa-6">
                <slot name="title">{{ dialogConfig.title }}</slot>
            </v-card-title>
            <v-card-text class="pa-6">
                <slot />
            </v-card-text>
            <v-card-actions class="pa-4 px-6">
                <v-spacer />
                <!--
                    The layout-only variant. Supplying #actions replaces the
                    cancel/confirm pair entirely, so a dialog with its own
                    buttons can still reuse the card, padding and sizing
                    instead of rebuilding the scaffold around a bare v-dialog.
                    `close` is exposed so a custom action can dismiss without
                    reaching for the model.
                -->
                <slot
                    name="actions"
                    :close="closeDialog"
                    :loading="loading"
                    :disabled="disabled"
                >
                    <v-btn
                        :variant="buttonConfigs.text.variant"
                        color="mutedForeground"
                        @click="closeDialog"
                    >
                        {{ resolvedCancelText }}
                    </v-btn>
                    <v-btn
                        v-if="showConfirmButton"
                        :color="buttonConfigs.primary.color"
                        :variant="buttonConfigs.primary.variant"
                        :loading="loading"
                        :disabled="disabled"
                        @click="confirmAction"
                    >
                        {{ resolvedConfirmText }}
                    </v-btn>
                </slot>
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>

<script setup lang="ts">
    import { computed } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import type { DialogConfig } from '@/types/schemas'
    import { getDialogMaxWidth, buttonConfigs } from '@/design-system/components'

    interface Props {
        config: DialogConfig
        loading?: boolean
        disabled?: boolean
        showConfirmButton?: boolean
        confirmText?: string
        cancelText?: string
    }
    const model = defineModel<boolean>({ required: true })

    // confirmText/cancelText deliberately have no literal default. They used
    // to default to 'Confirm' and 'Cancel', and five of the six callers pass
    // neither — so those dialogs showed English buttons in a German UI. The
    // fallback is now a translation, resolved below.
    const props = withDefaults(defineProps<Props>(), {
        loading: false,
        disabled: false,
        showConfirmButton: true,
        confirmText: undefined,
        cancelText: undefined,
    })

    const { t } = useTranslations()

    const emit = defineEmits<{ confirm: [] }>()

    const dialogConfig = computed(() => props.config)

    const resolvedConfirmText = computed(() => props.confirmText ?? t('common.confirm'))
    const resolvedCancelText = computed(() => props.cancelText ?? t('common.cancel'))

    const computedMaxWidth = computed(() => getDialogMaxWidth(dialogConfig.value.maxWidth))

    const closeDialog = () => {
        model.value = false
    }

    const confirmAction = () => {
        emit('confirm')
    }
</script>
