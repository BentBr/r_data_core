import { ref, computed, watch, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import { useFieldRendering } from '@/shared/composables/useFieldRendering'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import { getDialogMaxWidth, buttonConfigs } from '@/design-system/components'
import type { DynamicEntity, EntityDefinition, UpdateEntityRequest } from '@/types/schemas'

export default defineComponent({
    name: 'EntityEditDialog',
    components: {
        SmartIcon,
    },
    props: {
        modelValue: { type: Boolean, required: true },
        entity: { type: Object as PropType<DynamicEntity | null>, default: null },
        entityDefinition: { type: Object as PropType<EntityDefinition | null>, default: null },
        loading: { type: Boolean, default: false },
    },
    emits: ['update:modelValue', 'update'],
    setup(props, { emit }) {
        const { t } = useTranslations()
        const { getFieldComponent, getFieldRules, parseJsonFieldValue, stringifyJsonFieldValue } = useFieldRendering()

        const form = ref()
        const isValid = ref(false)
        const formData = ref<UpdateEntityRequest>({ data: { published: false }, parent_uuid: null })
        const fieldErrors = ref<Record<string, string>>({})

        const dialogVisible = computed({
            get: () => props.modelValue,
            set: value => emit('update:modelValue', value),
        })

        const availableParents = computed((): Array<{ title: string; uuid: string }> => [])
        const parentUuidValue = computed({
            get: () => formData.value.parent_uuid,
            set: (value: string | null | undefined) => { formData.value.parent_uuid = value ?? null },
        })

        const initializeFormData = () => {
            if (props.entity) {
                const data = { ...props.entity.field_data }
                if (data.published === undefined) data.published = false
                if (props.entityDefinition) {
                    for (const field of props.entityDefinition.fields) {
                        if (data[field.name] !== undefined) data[field.name] = stringifyJsonFieldValue(data[field.name], field.field_type)
                    }
                }
                
                formData.value = { data, parent_uuid: (props.entity.field_data.parent_uuid as string | undefined) ?? null }
                fieldErrors.value = {}
            }
        }

        const updateEntity = async () => {
            if (!form.value?.validate()) return
            fieldErrors.value = {}
            const processedData: Record<string, unknown> = { ...formData.value.data }
            if (props.entityDefinition) {
                for (const field of props.entityDefinition.fields) {
                    const fieldName = field.name
                    if (processedData[fieldName] !== undefined) {
                        const { parsed, error } = parseJsonFieldValue(processedData[fieldName], field.field_type)
                        if (error) fieldErrors.value[fieldName] = error
                        else processedData[fieldName] = parsed
                    }
                }
            }
            if (Object.keys(fieldErrors.value).length > 0) return
            emit('update', { data: processedData, parent_uuid: formData.value.parent_uuid })
        }

        const getFieldErrorMessages = (fieldName: string) => {
            const error = fieldErrors.value[fieldName]
            return error ? [error] : []
        }

        const setFieldErrors = (errors: Record<string, string>) => { fieldErrors.value = errors }

        watch(dialogVisible, visible => { if (visible && props.entity) initializeFormData(); else if (!visible) { dialogVisible.value = false; formData.value = { data: { published: false }, parent_uuid: undefined }; fieldErrors.value = {} } })
        watch(() => props.entity, entity => { if (entity && dialogVisible.value) initializeFormData() })
        watch(() => formData.value.data, () => { if (Object.keys(fieldErrors.value).length > 0) fieldErrors.value = {} }, { deep: true })

        return {
            t, getFieldComponent, getFieldRules, form, isValid, formData, fieldErrors,
            dialogVisible, availableParents, parentUuidValue, buttonConfigs,
            updateEntity, getFieldErrorMessages, setFieldErrors, closeDialog: () => { dialogVisible.value = false },
            getDialogMaxWidth,
        }
    },
})
