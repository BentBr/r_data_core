import { ref, watch, nextTick, defineComponent } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import { getDialogMaxWidth } from '@/design-system/components'
import type { CreateApiKeyRequest } from '@/types/schemas'

export default defineComponent({
    name: 'ApiKeyCreateDialog',
    props: {
        modelValue: { type: Boolean, required: true },
        loading: { type: Boolean, required: true },
    },
    emits: ['update:modelValue', 'create'],
    setup(props, { emit }) {
        const { t } = useTranslations()
        const createFormValid = ref(false)
        const createForm = ref<CreateApiKeyRequest>({ name: '', description: '', expires_in_days: undefined })
        const nameField = ref<HTMLInputElement | null>(null)

        const resetForm = () => { createForm.value = { name: '', description: '', expires_in_days: undefined }; createFormValid.value = false }
        const validateForm = () => { createFormValid.value = !!createForm.value.name.trim() }
        const createApiKey = () => { if (!createFormValid.value) return; emit('create', { ...createForm.value }); }
        const closeDialog = () => { emit('update:modelValue', false); resetForm() }
        const focusNameField = () => { void nextTick(() => { nameField.value?.focus() }) }

        watch(() => props.modelValue, val => { if (val) resetForm() })

        return { t, createFormValid, createForm, nameField, validateForm, createApiKey, closeDialog, focusNameField, getDialogMaxWidth, emit }
    },
})
