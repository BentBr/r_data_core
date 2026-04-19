import { onMounted, ref, defineComponent } from 'vue'
import { useSnackbar } from '@/shared/composables/useSnackbar'
import { useErrorHandler } from '@/shared/composables/useErrorHandler'
import { useTranslations } from '@/shared/composables/useTranslations'
import { typedHttpClient } from '@/api/typed-client'
import { useLicenseStore } from '@/stores/license'
import type { LicenseState } from '@/api/clients/system'
import PageLayout from '@/shared/components/PageLayout/index.vue'
import SnackbarManager from '@/shared/components/SnackbarManager/index.vue'

export default defineComponent({
    name: 'SystemPage',
    components: {
        PageLayout,
        SnackbarManager,
    },
    setup() {
        const { currentSnackbar, showSuccess } = useSnackbar()
        const { handleError } = useErrorHandler()
        const { t } = useTranslations()
        const licenseStore = useLicenseStore()

        const form = ref<{
            enabled: boolean
            max_versions: number | string | null
            max_age_days: number | string | null
        }>({
            enabled: true,
            max_versions: null,
            max_age_days: 180,
        })
        const loading = ref(false)
        const saving = ref(false)

        const runLogsForm = ref<{
            enabled: boolean
            max_runs: number | string | null
            max_age_days: number | string | null
        }>({
            enabled: true,
            max_runs: null,
            max_age_days: 90,
        })
        const loadingRunLogs = ref(false)
        const savingRunLogs = ref(false)

        const load = async () => {
            loading.value = true
            try {
                const settings = await typedHttpClient.getEntityVersioningSettings()
                form.value = {
                    enabled: settings.enabled,
                    max_versions: settings.max_versions ?? null,
                    max_age_days: settings.max_age_days ?? null,
                }
            } catch (err) {
                handleError(err)
            } finally {
                loading.value = false
            }
        }

        const save = async () => {
            saving.value = true
            try {
                const payload = {
                    enabled: form.value.enabled,
                    max_versions:
                        form.value.max_versions === null ||
                        form.value.max_versions === '' ||
                        Number.isNaN(Number(form.value.max_versions))
                            ? null
                            : Number(form.value.max_versions),
                    max_age_days:
                        form.value.max_age_days === null ||
                        form.value.max_age_days === '' ||
                        Number.isNaN(Number(form.value.max_age_days))
                            ? null
                            : Number(form.value.max_age_days),
                }
                await typedHttpClient.updateEntityVersioningSettings(payload)
                showSuccess(t('system.versioning.save_success'))
            } catch (err) {
                handleError(err)
            } finally {
                saving.value = false
            }
        }

        const loadRunLogs = async () => {
            loadingRunLogs.value = true
            try {
                const settings = await typedHttpClient.getWorkflowRunLogSettings()
                runLogsForm.value = {
                    enabled: settings.enabled,
                    max_runs: settings.max_runs ?? null,
                    max_age_days: settings.max_age_days ?? null,
                }
            } catch (err) {
                handleError(err)
            } finally {
                loadingRunLogs.value = false
            }
        }

        const saveRunLogs = async () => {
            savingRunLogs.value = true
            try {
                const payload = {
                    enabled: runLogsForm.value.enabled,
                    max_runs:
                        runLogsForm.value.max_runs === null ||
                        runLogsForm.value.max_runs === '' ||
                        Number.isNaN(Number(runLogsForm.value.max_runs))
                            ? null
                            : Number(runLogsForm.value.max_runs),
                    max_age_days:
                        runLogsForm.value.max_age_days === null ||
                        runLogsForm.value.max_age_days === '' ||
                        Number.isNaN(Number(runLogsForm.value.max_age_days))
                            ? null
                            : Number(runLogsForm.value.max_age_days),
                }
                await typedHttpClient.updateWorkflowRunLogSettings(payload)
                showSuccess(t('system.workflow_run_logs.save_success'))
            } catch (err) {
                handleError(err)
            } finally {
                savingRunLogs.value = false
            }
        }

        const getStateColor = (state: LicenseState): string => {
            switch (state) {
                case 'valid': return 'success'
                case 'invalid': return 'error'
                case 'error': return 'warning'
                case 'none': return 'error'
                default: return 'grey'
            }
        }

        const getStateLabel = (state: LicenseState): string => {
            switch (state) {
                case 'valid': return t('system.license.state_valid')
                case 'invalid': return t('system.license.state_invalid')
                case 'error': return t('system.license.state_error')
                case 'none': return t('system.license.state_none')
                default: return state
            }
        }

        const formatDate = (dateString: string | null | undefined): string => {
            if (!dateString) return '-'
            try {
                return new Date(dateString).toLocaleString()
            } catch {
                return dateString
            }
        }

        onMounted(() => {
            void load()
            void loadRunLogs()
            void licenseStore.loadLicenseStatus()
        })

        return {
            t,
            licenseStore,
            form,
            loading,
            saving,
            runLogsForm,
            loadingRunLogs,
            savingRunLogs,
            getStateColor,
            getStateLabel,
            formatDate,
            save,
            saveRunLogs,
            currentSnackbar,
        }
    },
})
