import { ref, computed, watch, defineComponent, PropType } from 'vue'
import type { SnackbarConfig } from '@/types/schemas'
import { buttonConfigs } from '@/design-system/components'
import { useSnackbar } from '@/shared/composables/useSnackbar'

export default defineComponent({
    name: 'SnackbarManager',
    props: {
        snackbar: { type: Object as PropType<SnackbarConfig | null>, default: null },
    },
    setup(props) {
        const { clearSnackbar } = useSnackbar()
        const showSnackbar = ref(false)
        const currentSnackbar = computed(() => {
            if (!props.snackbar) return { message: '', color: 'info', timeout: 3000, persistent: false }
            return { message: props.snackbar.message, color: props.snackbar.color ?? 'info', timeout: props.snackbar.timeout ?? 3000, persistent: props.snackbar.persistent ?? false }
        })
        const closeSnackbar = () => {
            showSnackbar.value = false
            setTimeout(() => { clearSnackbar() }, 300)
        }
        watch(() => props.snackbar, newSnackbar => { if (newSnackbar?.message) showSnackbar.value = true }, { immediate: true })
        watch(showSnackbar, newValue => { if (!newValue) setTimeout(() => { clearSnackbar() }, 300) })
        return { showSnackbar, currentSnackbar, buttonConfigs, closeSnackbar }
    },
})
