import { ref } from 'vue'
import type { DialogConfig } from '@/types/schemas'

export function useDialog() {
    const showDialog = ref(false)
    const dialogConfig = ref<DialogConfig>({
        title: '',
        maxWidth: '600px',
        persistent: false,
    })
    const loading = ref(false)
    const disabled = ref(false)

    const openDialog = (config: DialogConfig) => {
        dialogConfig.value = config
        showDialog.value = true
    }

    const closeDialog = () => {
        showDialog.value = false
        loading.value = false
        disabled.value = false
    }

    const setLoading = (value: boolean) => {
        loading.value = value
    }

    const setDisabled = (value: boolean) => {
        disabled.value = value
    }

    return {
        showDialog,
        dialogConfig,
        loading,
        disabled,
        openDialog,
        closeDialog,
        setLoading,
        setDisabled,
    }
}
