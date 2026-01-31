<template>
    <v-dialog
        :model-value="modelValue"
        :max-width="getDialogMaxWidth('default')"
        @update:model-value="$emit('update:modelValue', $event)"
        @after-enter="focusNameField"
    >
        <v-card>
            <v-card-title class="pa-6">{{ t('api_keys.create.title') }}</v-card-title>
            <v-card-text class="pa-6">
                <v-form
                    ref="createForm"
                    v-model="createFormValid"
                >
                    <v-text-field
                        ref="nameField"
                        v-model="createForm.name"
                        :label="t('api_keys.create.name_label')"
                        :rules="[v => !!v ?? t('api_keys.create.name_required')]"
                        required
                        @input="validateForm"
                    />
                    <v-textarea
                        v-model="createForm.description"
                        :label="t('api_keys.create.description_label')"
                        rows="3"
                    />
                    <v-text-field
                        v-model.number="createForm.expires_in_days"
                        :label="t('api_keys.create.expires_label')"
                        type="number"
                        min="1"
                        max="3650"
                        :hint="t('api_keys.create.expires_hint')"
                    />
                </v-form>
            </v-card-text>
            <v-card-actions class="pa-4 px-6">
                <v-spacer />
                <v-btn
                    variant="text"
                    color="mutedForeground"
                    @click="closeDialog"
                >
                    {{ t('common.cancel') }}
                </v-btn>
                <v-btn
                    color="primary"
                    variant="flat"
                    :loading="loading"
                    :disabled="!createFormValid"
                    @click="createApiKey"
                >
                    {{ t('api_keys.create.button') }}
                </v-btn>
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>

<script setup lang="ts">
    import { ref, watch, nextTick } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import { getDialogMaxWidth } from '@/design-system/components'
    import type { CreateApiKeyRequest } from '@/types/schemas'

    const { t } = useTranslations()

    // Props
    interface Props {
        modelValue: boolean
        loading: boolean
    }

    const props = defineProps<Props>()

    // Emits
    const emit = defineEmits<{
        'update:modelValue': [value: boolean]
        create: [data: CreateApiKeyRequest]
    }>()

    // Reactive state
    const createFormValid = ref(false)
    const createForm = ref<CreateApiKeyRequest>({
        name: '',
        description: '',
        expires_in_days: undefined,
    })

    // Refs for form validation
    const nameField = ref<HTMLInputElement | null>(null)

    // Watch for dialog state changes
    watch(
        () => props.modelValue,
        newValue => {
            if (newValue) {
                // Reset form when dialog opens
                resetForm()
            }
        }
    )

    // Methods
    const resetForm = () => {
        createForm.value = {
            name: '',
            description: '',
            expires_in_days: undefined,
        }
        createFormValid.value = false
    }

    const validateForm = () => {
        createFormValid.value = !!createForm.value.name.trim()
    }

    const createApiKey = () => {
        if (!createFormValid.value) {
            return
        }

        // Create a clean object without circular references
        const requestData: CreateApiKeyRequest = {
            name: createForm.value.name,
            description: createForm.value.description ?? undefined,
            expires_in_days: createForm.value.expires_in_days ?? undefined,
        }

        emit('create', requestData)
    }

    const closeDialog = () => {
        emit('update:modelValue', false)
        resetForm()
    }

    const focusNameField = () => {
        // Use nextTick to ensure the field is rendered
        void nextTick(() => {
            if (nameField.value) {
                nameField.value.focus()
            }
        })
    }
</script>
