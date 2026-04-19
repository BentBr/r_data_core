import { ref, watch, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import { useRoles } from '@/modules/permissions/composables/useRoles'
import { getDialogMaxWidth } from '@/design-system/components'
import type { UserResponse, CreateUserRequest, UpdateUserRequest, Role } from '@/types/schemas'

export default defineComponent({
    name: 'UserDialog',
    props: {
        modelValue: { type: Boolean, required: true },
        user: { type: Object as PropType<UserResponse | null>, default: null },
        loading: { type: Boolean, required: true },
    },
    emits: ['update:modelValue', 'save'],
    setup(props, { emit }) {
        const { t } = useTranslations()
        const { loadRoles, roles, loading: loadingRoles } = useRoles()
        const formRef = ref()
        const formValid = ref(false)
        const availableRoles = ref<Role[]>([])
        const formData = ref<CreateUserRequest & { is_active?: boolean, super_admin?: boolean }>({ username: '', email: '', password: '', first_name: '', last_name: '', role_uuids: [], is_active: true, super_admin: false })

        const rules = {
            required: (v: string) => !!v || t('users.dialog.validation.required'),
            email: (v: string) => !v || /.+@.+\..+/.test(v) || t('users.dialog.validation.email_invalid'),
            minLength: (min: number) => (v: string) => !v || v.length >= min || t('users.dialog.validation.min_length', { min: String(min) }),
        }

        const resetForm = () => { formData.value = { username: '', email: '', password: '', first_name: '', last_name: '', role_uuids: [], is_active: true, super_admin: false }; formValid.value = false }
        const loadAvailableRoles = async () => { try { await loadRoles(1, 100); availableRoles.value = roles.value } catch (err) { console.error('Failed to load roles:', err) } }

        watch(() => props.modelValue, async val => { if (val) await loadAvailableRoles(); else resetForm() })
        watch(() => props.user, newUser => {
            if (newUser) formData.value = { username: newUser.username, email: newUser.email, password: '', first_name: newUser.first_name ?? '', last_name: newUser.last_name ?? '', role_uuids: newUser.role_uuids, is_active: newUser.is_active, super_admin: newUser.super_admin }
            else resetForm()
        }, { immediate: true })

        const handleSave = () => {
            if (!formValid.value) return
            if (props.user) {
                const data: UpdateUserRequest = { email: formData.value.email, first_name: formData.value.first_name, last_name: formData.value.last_name, role_uuids: formData.value.role_uuids ?? undefined, is_active: formData.value.is_active, super_admin: formData.value.super_admin }
                if (formData.value.password) data.password = formData.value.password
                emit('save', data)
            } else emit('save', { ...formData.value } as CreateUserRequest)
        }

        return { t, loadingRoles, formRef, formValid, availableRoles, formData, rules, handleClose: () => { emit('update:modelValue', false); resetForm() }, handleSave, getDialogMaxWidth, emit }
    },
})
