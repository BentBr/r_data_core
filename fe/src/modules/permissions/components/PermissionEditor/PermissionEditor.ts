import { computed, watch, defineComponent, PropType } from 'vue'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import type { Permission, ResourceNamespace, PermissionType, AccessLevel } from '@/types/schemas'

export default defineComponent({
    name: 'PermissionEditor',
    components: {
        SmartIcon,
    },
    props: {
        permission: { type: Object as PropType<Permission>, required: true },
        resourceTypes: { type: Array as PropType<ResourceNamespace[]>, required: true },
        permissionTypes: { type: Array as PropType<PermissionType[]>, required: true },
        accessLevels: { type: Array as PropType<AccessLevel[]>, required: true },
        disabled: { type: Boolean, default: false },
    },
    emits: ['update', 'remove'],
    setup(props, { emit }) {
        const { t } = useTranslations()
        const filteredPermissionTypes = computed(() => {
            const isWorkflows = (props.permission.resource_type as ResourceNamespace) === 'Workflows'
            return props.permissionTypes.filter(type => type === 'Execute' ? isWorkflows : true).map(type => ({
                title: t(`permissions.types.${type.toLowerCase()}`) || type, value: type
            }))
        })
        watch(() => props.permission.resource_type, newResourceType => {
            if (props.permission.permission_type === 'Execute' && (newResourceType as ResourceNamespace) !== 'Workflows') {
                emit('update', { ...props.permission, permission_type: 'Read' as PermissionType })
            }
        })
        const updateField = (field: keyof Permission, value: unknown) => { emit('update', { ...props.permission, [field]: value }) }
        return { t, filteredPermissionTypes, updateField, emit }
    },
})
