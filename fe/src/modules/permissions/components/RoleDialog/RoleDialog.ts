import { ref, watch, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import { getDialogMaxWidth } from '@/design-system/components'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import PermissionEditor from '../PermissionEditor/index.vue'
import type { Role, Permission, ResourceNamespace, PermissionType, AccessLevel } from '@/types/schemas'

export default defineComponent({
    name: 'RoleDialog',
    components: {
        SmartIcon,
        PermissionEditor,
    },
    props: {
        modelValue: { type: Boolean, required: true },
        role: { type: Object as PropType<Role | null>, default: null },
        loading: { type: Boolean, default: false },
        resourceTypes: { type: Array as PropType<ResourceNamespace[]>, required: true },
        permissionTypes: { type: Array as PropType<PermissionType[]>, required: true },
        accessLevels: { type: Array as PropType<AccessLevel[]>, required: true },
    },
    emits: ['update:modelValue', 'save'],
    setup(props, { emit }) {
        const { t } = useTranslations()
        const formRef = ref()
        const formValid = ref(false)
        const formData = ref<{ name: string, description: string | null, permissions: Permission[], super_admin: boolean }>({ name: '', description: null, permissions: [], super_admin: false })

        const rules = { required: (v: string) => !!v || 'Required' }

        watch(() => [props.modelValue, props.role], () => {
            if (props.modelValue) {
                if (props.role) formData.value = { name: props.role.name, description: props.role.description ?? null, permissions: JSON.parse(JSON.stringify(props.role.permissions)), super_admin: props.role.super_admin }
                else formData.value = { name: '', description: null, permissions: [], super_admin: false }
            }
        }, { immediate: true })

        const addPermission = () => { formData.value.permissions.push({ resource_type: 'Workflows', permission_type: 'Read', access_level: 'All', resource_uuids: [], constraints: undefined }) }
        const removePermission = (index: number) => { formData.value.permissions.splice(index, 1) }
        const updatePermission = (index: number, permission: Permission) => { formData.value.permissions[index] = permission }
        const handleSave = () => { if (formValid.value) emit('save', { ...formData.value }) }

        return { t, formRef, formValid, formData, rules, addPermission, removePermission, updatePermission, handleSave, handleClose: () => { emit('update:modelValue', false) }, getDialogMaxWidth, emit }
    },
})
