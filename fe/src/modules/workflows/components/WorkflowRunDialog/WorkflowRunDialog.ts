import { ref, computed, defineComponent } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import { typedHttpClient } from '@/api/typed-client'
import { useSnackbar } from '@/shared/composables/useSnackbar'
import { useErrorHandler } from '@/shared/composables/useErrorHandler'
import { getDialogMaxWidth } from '@/design-system/components'

export default defineComponent({
    name: 'WorkflowRunDialog',
    props: {
        modelValue: { type: Boolean, required: true },
        workflowUuid: { type: String, default: null },
    },
    emits: ['update:modelValue', 'enqueued'],
    setup(props, { emit }) {
        const { t } = useTranslations()
        const { showSuccess } = useSnackbar()
        const { handleError } = useErrorHandler()

        const showDialog = computed({
            get: () => props.modelValue,
            set: (val) => emit('update:modelValue', val)
        })

        const uploadEnabled = ref(false)
        const uploadFile = ref<File | null>(null)
        const loading = ref(false)

        function onFileChange(e: Event) {
            const input = e.target as HTMLInputElement | null
            const files = input?.files
            uploadFile.value = files?.length ? files[0] : null
        }

        async function confirmRun() {
            if (!props.workflowUuid) return
            loading.value = true
            try {
                if (uploadEnabled.value && uploadFile.value) {
                    const res = await typedHttpClient.uploadRunFile(
                        props.workflowUuid,
                        uploadFile.value
                    )
                    showSuccess(`Run enqueued (staged ${res.staged_items})`)
                } else {
                    await typedHttpClient.runWorkflow(props.workflowUuid)
                    showSuccess('Workflow run enqueued')
                }
                emit('enqueued')
                showDialog.value = false
            } catch (e) {
                handleError(e)
            } finally {
                loading.value = false
            }
        }

        return {
            t,
            showDialog,
            uploadEnabled,
            uploadFile,
            loading,
            onFileChange,
            confirmRun,
            getDialogMaxWidth,
        }
    }
})
