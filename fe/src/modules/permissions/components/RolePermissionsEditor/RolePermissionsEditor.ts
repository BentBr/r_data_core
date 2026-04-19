import { defineComponent, PropType } from 'vue'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import PermissionEditor from '../PermissionEditor/index.vue'
import type {
    Permission,
    ResourceNamespace,
    PermissionType,
    AccessLevel,
} from '@/types/schemas'

export default defineComponent({
    name: 'RolePermissionsEditor',
    components: {
        SmartIcon,
        PermissionEditor,
    },
    props: {
        roleName: { type: String, required: true },
        permissions: { type: Array as PropType<Permission[]>, required: true },
        resourceTypes: { type: Array as PropType<ResourceNamespace[]>, required: true },
        permissionTypes: { type: Array as PropType<PermissionType[]>, required: true },
        accessLevels: { type: Array as PropType<AccessLevel[]>, required: true },
    },
    emits: ['update:roleName', 'update:permissions', 'add-permission', 'remove'],
    setup(props, { emit }) {
        const updatePermission = (index: number, permission: Permission) => {
            const updated = [...props.permissions]
            updated[index] = permission
            emit('update:permissions', updated)
        }

        const removePermission = (index: number) => {
            const updated = [...props.permissions]
            updated.splice(index, 1)
            emit('update:permissions', updated)
        }

        return {
            updatePermission, removePermission, emit,
        }
    },
})
