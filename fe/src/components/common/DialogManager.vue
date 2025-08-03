<template>
    <v-dialog
        v-model="showDialog"
        :max-width="dialogConfig.maxWidth || '600px'"
        :persistent="dialogConfig.persistent || false"
    >
        <v-card>
            <v-card-title class="text-h5 pa-4">
                {{ dialogConfig.title }}
            </v-card-title>
            <v-card-text>
                <slot />
            </v-card-text>
            <v-card-actions class="pa-4">
                <v-spacer />
                <v-btn
                    color="grey"
                    variant="text"
                    @click="closeDialog"
                >
                    {{ cancelText }}
                </v-btn>
                <v-btn
                    v-if="showConfirmButton"
                    color="primary"
                    :loading="loading"
                    :disabled="disabled"
                    @click="confirmAction"
                >
                    {{ confirmText }}
                </v-btn>
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>

<script setup lang="ts">
    import { computed } from 'vue'
    import type { DialogConfig } from '@/types/schemas'

    interface Props {
        modelValue: boolean
        config: DialogConfig
        loading?: boolean
        disabled?: boolean
        showConfirmButton?: boolean
        confirmText?: string
        cancelText?: string
    }

    interface Emits {
        (e: 'update:modelValue', value: boolean): void
        (e: 'confirm'): void
    }

    const props = withDefaults(defineProps<Props>(), {
        loading: false,
        disabled: false,
        showConfirmButton: true,
        confirmText: 'Confirm',
        cancelText: 'Cancel',
    })

    const emit = defineEmits<Emits>()

    const showDialog = computed({
        get: () => props.modelValue,
        set: value => emit('update:modelValue', value),
    })

    const dialogConfig = computed(() => props.config)

    const closeDialog = () => {
        showDialog.value = false
    }

    const confirmAction = () => {
        emit('confirm')
    }
</script>
