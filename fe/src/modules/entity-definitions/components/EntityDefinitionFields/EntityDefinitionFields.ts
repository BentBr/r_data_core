import { computed, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import Badge from '@/shared/components/Badge/index.vue'
import type { EntityDefinition } from '@/types/schemas'

export default defineComponent({
    name: 'EntityDefinitionFields',
    components: {
        SmartIcon,
        Badge,
    },
    props: {
        definition: {
            type: Object as PropType<EntityDefinition>,
            required: true,
        },
        hasUnsavedChanges: {
            type: Boolean,
            required: true,
        },
        savingChanges: {
            type: Boolean,
            required: true,
        },
        loading: {
            type: Boolean,
            default: false,
        },
    },
    emits: ['save-changes', 'add-field', 'edit-field', 'remove-field'],
    setup(props, { emit }) {
        const { t } = useTranslations()

        const fieldTreeItems = computed(() => {
            return props.definition.fields.map(field => ({
                ...field,
            }))
        })

        const getFieldIcon = (fieldType: string) => {
            const iconMap: Record<string, string> = {
                String: 'type', Text: 'file-text', Wysiwyg: 'file-edit',
                Integer: 'hash', Float: 'hash', Boolean: 'check-square',
                Date: 'calendar', DateTime: 'calendar-clock', Object: 'box',
                Array: 'list', Json: 'braces', Uuid: 'hash', ManyToOne: 'link',
                ManyToMany: 'link-2', Select: 'list-checks', MultiSelect: 'list-checks',
                Image: 'image', File: 'file', Password: 'lock',
            }
            return iconMap[fieldType] || 'type'
        }

        const getFieldColor = (fieldType: string) => {
            const colorMap: Record<string, string> = {
                String: 'primary', Text: 'primary', Wysiwyg: 'primary',
                Integer: 'success', Float: 'success', Boolean: 'warning',
                Date: 'info', DateTime: 'info', Object: 'purple',
                Array: 'orange', Json: 'teal', Uuid: 'grey', ManyToOne: 'blue',
                ManyToMany: 'blue', Select: 'green', MultiSelect: 'green',
                Image: 'pink', File: 'brown', Password: 'red',
            }
            return colorMap[fieldType] || 'primary'
        }

        const getFieldTypeDisplayName = (fieldType: string) => {
            const displayNameMap: Record<string, string> = {
                Object: 'Json Object',
                Array: 'Json Array',
                Json: 'Json (any)',
            }
            return displayNameMap[fieldType] || fieldType
        }

        return {
            t, fieldTreeItems, getFieldIcon, getFieldColor, getFieldTypeDisplayName, emit,
        }
    },
})
