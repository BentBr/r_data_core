import { ref, computed, defineComponent } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import IconPicker from '@/shared/components/IconPicker/index.vue'
import { getDialogMaxWidth } from '@/design-system/components'
import type { CreateEntityDefinitionRequest } from '@/types/schemas'

export default defineComponent({
    name: 'EntityDefinitionCreateDialog',
    components: {
        IconPicker,
    },
    props: {
        modelValue: { type: Boolean, required: true },
        loading: { type: Boolean, default: false },
    },
    emits: ['update:modelValue', 'create'],
    setup(props, { emit }) {
        const { t } = useTranslations()
        const showDialog = computed({ get: () => props.modelValue, set: v => emit('update:modelValue', v) })
        const formValid = ref(false)
        const formRef = ref<HTMLFormElement | null>(null)
        const form = ref<CreateEntityDefinitionRequest>({ entity_type: '', display_name: '', description: '', group_name: '', allow_children: false, icon: '', fields: [], published: false })

        const resetForm = () => { form.value = { entity_type: '', display_name: '', description: '', group_name: '', allow_children: false, icon: '', fields: [], published: false } }
        const closeDialog = () => { showDialog.value = false; resetForm() }
        const createEntityDefinition = () => { if (formValid.value) { emit('create', { ...form.value }); closeDialog() } }

        return { t, showDialog, formValid, formRef, form, closeDialog, createEntityDefinition, getDialogMaxWidth }
    },
})
