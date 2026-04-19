import { ref, computed, watch, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import IconPicker from '@/shared/components/IconPicker/index.vue'
import { getDialogMaxWidth } from '@/design-system/components'
import type { EntityDefinition, UpdateEntityDefinitionRequest } from '@/types/schemas'

export default defineComponent({
    name: 'EntityDefinitionEditDialog',
    components: {
        IconPicker,
    },
    props: {
        modelValue: { type: Boolean, required: true },
        definition: { type: Object as PropType<EntityDefinition | null>, default: null },
        loading: { type: Boolean, default: false },
    },
    emits: ['update:modelValue', 'update'],
    setup(props, { emit }) {
        const { t } = useTranslations()
        const showDialog = computed({ get: () => props.modelValue, set: v => emit('update:modelValue', v) })
        const formValid = ref(false)
        const formRef = ref<HTMLFormElement | null>(null)
        const form = ref<UpdateEntityDefinitionRequest>({ entity_type: '', display_name: '', description: '', group_name: '', allow_children: false, icon: '', fields: [], published: false })

        const closeDialog = () => { showDialog.value = false }
        const updateEntityDefinition = () => { if (formValid.value) { emit('update', { ...form.value }); closeDialog() } }

        watch(() => props.definition, def => {
            if (def) form.value = { entity_type: def.entity_type, display_name: def.display_name, description: def.description ?? '', group_name: def.group_name ?? '', allow_children: def.allow_children, icon: def.icon ?? '', fields: [...def.fields], published: def.published ?? false }
        }, { immediate: true })

        return { t, showDialog, formValid, formRef, form, closeDialog, updateEntityDefinition, getDialogMaxWidth }
    },
})
