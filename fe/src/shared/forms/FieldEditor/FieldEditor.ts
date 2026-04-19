import { ref, computed, watch, defineComponent, PropType } from 'vue'
import type { FieldDefinition } from '@/types/schemas'
import { useTranslations } from '@/shared/composables/useTranslations'
import { getDialogMaxWidth } from '@/design-system/components'

export default defineComponent({
    name: 'FieldEditor',
    props: {
        modelValue: {
            type: Boolean,
            required: true,
        },
        field: {
            type: Object as PropType<FieldDefinition>,
            default: undefined,
        },
    },
    emits: ['update:modelValue', 'save'],
    setup(props, { emit }) {
        const { t } = useTranslations()

        const showDialog = computed({
            get: () => props.modelValue,
            set: value => emit('update:modelValue', value),
        })

        const isEditing = computed(() => !!props.field)
        const formValid = ref(false)
        const formRef = ref<HTMLFormElement | null>(null)

        const fieldTypes = [
            { title: 'String', value: 'String' }, { title: 'Text', value: 'Text' },
            { title: 'Wysiwyg', value: 'Wysiwyg' }, { title: 'Integer', value: 'Integer' },
            { title: 'Float', value: 'Float' }, { title: 'Boolean', value: 'Boolean' },
            { title: 'Date', value: 'Date' }, { title: 'DateTime', value: 'DateTime' },
            { title: 'Json (any)', value: 'Json' }, { title: 'Json Object', value: 'Object' },
            { title: 'Json Array', value: 'Array' }, { title: 'Uuid', value: 'Uuid' },
            { title: 'ManyToOne', value: 'ManyToOne' }, { title: 'ManyToMany', value: 'ManyToMany' },
            { title: 'Select', value: 'Select' }, { title: 'MultiSelect', value: 'MultiSelect' },
            { title: 'Image', value: 'Image' }, { title: 'File', value: 'File' },
            { title: 'Password', value: 'Password' },
        ]

        const form = ref<FieldDefinition>({
            name: '', display_name: '', field_type: 'String', description: '',
            required: false, indexed: false, filterable: false, unique: false,
            default_value: undefined, constraints: {}, ui_settings: {},
        })

        const showDefaultValue = computed(() => {
            if (form.value.field_type === 'Password') return false
            return ['String', 'Text', 'Integer', 'Float', 'Boolean', 'Date', 'DateTime'].includes(form.value.field_type)
        })

        const EMAIL_REGEX = '^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$'
        const isStringType = computed(() => ['String', 'Text', 'Wysiwyg', 'Password'].includes(form.value.field_type))
        const isNumericType = computed(() => ['Integer', 'Float'].includes(form.value.field_type))
        const supportsUniqueness = computed(() => ['String', 'Text', 'Integer', 'Uuid'].includes(form.value.field_type))
        const showValidationSection = computed(() => isStringType.value || isNumericType.value || supportsUniqueness.value)

        const emailPreset = computed({
            get: () => form.value.constraints?.pattern === EMAIL_REGEX,
            set: (value: boolean) => {
                form.value.constraints ??= {}
                form.value.constraints.pattern = value ? EMAIL_REGEX : undefined
            },
        })

        const ensureConstraints = () => { form.value.constraints ??= {} }

        const constraintMinLength = computed({
            get: () => form.value.constraints?.min_length as number | undefined,
            set: (value: number | undefined) => { ensureConstraints(); form.value.constraints!.min_length = value },
        })
        const constraintMaxLength = computed({
            get: () => form.value.constraints?.max_length as number | undefined,
            set: (value: number | undefined) => { ensureConstraints(); form.value.constraints!.max_length = value },
        })
        const constraintPattern = computed({
            get: () => form.value.constraints?.pattern as string | undefined,
            set: (value: string | undefined) => { ensureConstraints(); form.value.constraints!.pattern = value },
        })
        const constraintMin = computed({
            get: () => form.value.constraints?.min as number | undefined,
            set: (value: number | undefined) => { ensureConstraints(); form.value.constraints!.min = value },
        })
        const constraintMax = computed({
            get: () => form.value.constraints?.max as number | undefined,
            set: (value: number | undefined) => { ensureConstraints(); form.value.constraints!.max = value },
        })
        const constraintPositiveOnly = computed({
            get: () => form.value.constraints?.positive_only as boolean | undefined,
            set: (value: boolean | undefined) => { ensureConstraints(); form.value.constraints!.positive_only = value },
        })
        const constraintUnique = computed({
            get: () => form.value.unique ?? false,
            set: (value: boolean) => { form.value.unique = value },
        })

        const resetForm = () => {
            form.value = {
                name: '', display_name: '', field_type: 'String', description: '',
                required: false, indexed: false, filterable: false, unique: false,
                default_value: undefined, constraints: {}, ui_settings: {},
            }
        }

        watch(() => props.field, newField => {
            if (newField) {
                const apiConstraints = newField.constraints as any
                const innerConstraints = apiConstraints?.constraints ?? {}
                form.value = {
                    name: newField.name, display_name: newField.display_name, field_type: newField.field_type,
                    description: newField.description ?? '', required: newField.required,
                    indexed: newField.indexed, filterable: newField.filterable,
                    unique: newField.unique ?? false, default_value: newField.default_value,
                    constraints: innerConstraints, ui_settings: newField.ui_settings ?? {},
                }
            } else resetForm()
        }, { immediate: true })

        const formatDefaultValue = (value: unknown, fieldType: string): unknown => {
            if (value === null || value === undefined || value === '') return undefined
            switch (fieldType) {
                case 'Boolean':
                    if (typeof value === 'boolean') return value
                    if (typeof value === 'string') return value.toLowerCase() === 'true'
                    return !!value
                case 'Integer': return typeof value === 'number' ? Math.floor(value) : parseInt(String(value), 10) || undefined
                case 'Float': return typeof value === 'number' ? value : parseFloat(String(value)) || undefined
                case 'Date':
                case 'DateTime': return typeof value === 'string' ? value : undefined
                case 'Object':
                case 'Array':
                case 'Json': return typeof value === 'object' ? value : (typeof value === 'string' ? JSON.parse(value) : undefined)
                default: return String(value)
            }
        }

        const getConstraintType = (fieldType: string): string => {
            switch (fieldType) {
                case 'String': case 'Text': case 'Wysiwyg': case 'Password': return 'string'
                case 'Integer': return 'integer'
                case 'Float': return 'float'
                case 'DateTime': return 'datetime'
                case 'Date': return 'date'
                case 'Select': return 'select'
                case 'MultiSelect': return 'multiselect'
                case 'ManyToOne': case 'ManyToMany': return 'relation'
                default: return 'schema'
            }
        }

        const saveField = () => {
            if (!formValid.value) return
            const formattedDefaultValue = form.value.default_value !== undefined ? formatDefaultValue(form.value.default_value, form.value.field_type) : undefined
            const formattedConstraints = { type: getConstraintType(form.value.field_type), constraints: form.value.constraints ?? {} }
            const sanitizedField = { ...form.value, unique: form.value.unique ?? false, default_value: formattedDefaultValue, constraints: formattedConstraints, ui_settings: form.value.ui_settings ?? {} }
            emit('save', sanitizedField)
            showDialog.value = false
        }

        return {
            t, showDialog, isEditing, formValid, formRef, fieldTypes, form, showDefaultValue,
            isStringType, isNumericType, supportsUniqueness, showValidationSection, emailPreset,
            constraintMinLength, constraintMaxLength, constraintPattern, constraintMin, constraintMax,
            constraintPositiveOnly, constraintUnique, closeDialog: () => { showDialog.value = false },
            saveField, getDialogMaxWidth,
        }
    },
})
