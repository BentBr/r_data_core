import { computed, defineComponent, PropType } from 'vue'
import type { DialogConfig } from '@/types/schemas'
import { getDialogMaxWidth, buttonConfigs } from '@/design-system/components'

export default defineComponent({
    name: 'DialogManager',
    props: {
        modelValue: { type: Boolean, required: true },
        config: { type: Object as PropType<DialogConfig>, required: true },
        loading: { type: Boolean, default: false },
        disabled: { type: Boolean, default: false },
        showConfirmButton: { type: Boolean, default: true },
        confirmText: { type: String, default: 'Confirm' },
        cancelText: { type: String, default: 'Cancel' },
    },
    emits: ['update:modelValue', 'confirm'],
    setup(props, { emit }) {
        const showDialog = computed({
            get: () => props.modelValue,
            set: value => emit('update:modelValue', value),
        })
        const dialogConfig = computed(() => props.config)
        const computedMaxWidth = computed(() => dialogConfig.value.maxWidth ?? getDialogMaxWidth('default'))
        return {
            showDialog, dialogConfig, computedMaxWidth, buttonConfigs,
            closeDialog: () => { showDialog.value = false },
            confirmAction: () => { emit('confirm') },
        }
    },
})
