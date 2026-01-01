<template>
    <div>
        <PageLayout>
            <v-row>
                <v-col
                    cols="12"
                    md="6"
                >
                    <v-card variant="outlined">
                        <v-card-title class="text-subtitle-1 pa-3">
                            {{ t('system.license.section_title') }}
                        </v-card-title>
                        <v-card-text class="pa-3">
                            <div v-if="licenseStore.licenseStatus">
                                <v-row>
                                    <v-col cols="6">
                                        <strong>{{ t('system.license.state') }}:</strong>
                                    </v-col>
                                    <v-col cols="6">
                                        <v-chip
                                            :color="getStateColor(licenseStore.licenseStatus.state)"
                                            size="small"
                                        >
                                            {{ getStateLabel(licenseStore.licenseStatus.state) }}
                                        </v-chip>
                                    </v-col>
                                </v-row>
                                <v-row v-if="licenseStore.licenseStatus.company">
                                    <v-col cols="6">
                                        <strong>{{ t('system.license.company') }}:</strong>
                                    </v-col>
                                    <v-col cols="6">
                                        {{ licenseStore.licenseStatus.company }}
                                    </v-col>
                                </v-row>
                                <v-row v-if="licenseStore.licenseStatus.license_type">
                                    <v-col cols="6">
                                        <strong>{{ t('system.license.license_type') }}:</strong>
                                    </v-col>
                                    <v-col cols="6">
                                        {{ licenseStore.licenseStatus.license_type }}
                                    </v-col>
                                </v-row>
                                <v-row v-if="licenseStore.licenseStatus.license_id">
                                    <v-col cols="6">
                                        <strong>{{ t('system.license.license_id') }}:</strong>
                                    </v-col>
                                    <v-col cols="6">
                                        <code>{{ licenseStore.licenseStatus.license_id }}</code>
                                    </v-col>
                                </v-row>
                                <v-row v-if="licenseStore.licenseStatus.issued_at">
                                    <v-col cols="6">
                                        <strong>{{ t('system.license.issued_at') }}:</strong>
                                    </v-col>
                                    <v-col cols="6">
                                        {{ formatDate(licenseStore.licenseStatus.issued_at) }}
                                    </v-col>
                                </v-row>
                                <v-row v-if="licenseStore.licenseStatus.version">
                                    <v-col cols="6">
                                        <strong>{{ t('system.license.version') }}:</strong>
                                    </v-col>
                                    <v-col cols="6">
                                        {{ licenseStore.licenseStatus.version }}
                                    </v-col>
                                </v-row>
                                <v-row v-if="licenseStore.licenseStatus.error_message">
                                    <v-col cols="12">
                                        <v-alert
                                            type="error"
                                            variant="tonal"
                                            density="compact"
                                        >
                                            {{ licenseStore.licenseStatus.error_message }}
                                        </v-alert>
                                    </v-col>
                                </v-row>
                            </div>
                            <div v-else>
                                <v-skeleton-loader type="text" />
                            </div>
                        </v-card-text>
                    </v-card>
                </v-col>
                <v-col
                    cols="12"
                    md="6"
                >
                    <v-card variant="outlined">
                        <v-card-title class="text-subtitle-1 pa-3">
                            {{ t('system.versioning.section_title') }}
                        </v-card-title>
                        <v-card-text class="pa-3">
                            <v-switch
                                v-model="form.enabled"
                                :label="t('system.versioning.enabled')"
                                color="success"
                                inset
                            />
                            <v-text-field
                                v-model="form.max_versions"
                                :label="t('system.versioning.max_versions')"
                                type="number"
                                min="0"
                            />
                            <v-text-field
                                v-model="form.max_age_days"
                                :label="t('system.versioning.max_age_days')"
                                type="number"
                                min="0"
                            />
                        </v-card-text>
                        <v-card-actions>
                            <v-spacer />
                            <v-btn
                                color="primary"
                                variant="flat"
                                :loading="saving"
                                @click="save"
                            >
                                {{ t('system.versioning.save') }}
                            </v-btn>
                        </v-card-actions>
                    </v-card>
                </v-col>
            </v-row>

            <SnackbarManager :snackbar="currentSnackbar" />
        </PageLayout>
    </div>
</template>

<script setup lang="ts">
    import { onMounted, ref } from 'vue'
    import { useSnackbar } from '@/composables/useSnackbar'
    import { useErrorHandler } from '@/composables/useErrorHandler'
    import { useTranslations } from '@/composables/useTranslations'
    import { typedHttpClient } from '@/api/typed-client'
    import { useLicenseStore } from '@/stores/license'
    import type { LicenseState } from '@/api/clients/system'
    import PageLayout from '@/components/layouts/PageLayout.vue'
    import SnackbarManager from '@/components/common/SnackbarManager.vue'

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

    const load = async () => {
        loading.value = true
        try {
            const settings = await typedHttpClient.getEntityVersioningSettings()
            if (settings) {
                form.value = {
                    enabled: settings.enabled,
                    max_versions: settings.max_versions ?? null,
                    max_age_days: settings.max_age_days ?? null,
                }
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
            // Convert string inputs to numbers or null
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

    const getStateColor = (state: LicenseState): string => {
        switch (state) {
            case 'valid':
                return 'success'
            case 'invalid':
                return 'error'
            case 'error':
                return 'warning'
            case 'none':
                return 'error'
            default:
                return 'grey'
        }
    }

    const getStateLabel = (state: LicenseState): string => {
        switch (state) {
            case 'valid':
                return t('system.license.state_valid')
            case 'invalid':
                return t('system.license.state_invalid')
            case 'error':
                return t('system.license.state_error')
            case 'none':
                return t('system.license.state_none')
            default:
                return state
        }
    }

    const formatDate = (dateString: string | null | undefined): string => {
        if (!dateString) {
            return '-'
        }
        try {
            return new Date(dateString).toLocaleString()
        } catch {
            return dateString
        }
    }

    onMounted(() => {
        void load()
        void licenseStore.loadLicenseStatus()
    })
</script>
