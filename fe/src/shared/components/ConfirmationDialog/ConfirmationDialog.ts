import { computed, defineComponent } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import { getDialogMaxWidth, buttonConfigs } from '@/design-system/components'

export default defineComponent({
    name: 'ConfirmationDialog',
    props: {
        modelValue: { type: Boolean, required: true },
        title: { type: String, required: true },
        confirmText: { type: String, default: undefined },
        cancelText: { type: String, default: undefined },
        loading: { type: Boolean, default: false },
        color: { type: String, default: 'primary' },
    },
    emits: ['update:modelValue', 'confirm', 'cancel'],
    setup(props, { emit }) {
        const { t } = useTranslations()
        const show = computed({
            get: () => props.modelValue,
            set: value => emit('update:modelValue', value),
        })
        const handleCancel = () => { show.value = false; emit('cancel') }
        const handleConfirm = () => { emit('confirm') }
        return { t, show, handleCancel, handleConfirm, buttonConfigs, getDialogMaxWidth }
    },
})
