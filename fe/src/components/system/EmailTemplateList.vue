<template>
    <div data-testid="email-template-list">
        <div class="d-flex align-center justify-space-between pa-4">
            <h3 class="text-h6">{{ t('system.email_templates.title') }}</h3>
            <v-btn
                color="primary"
                @click="openCreate"
            >
                <template #prepend>
                    <SmartIcon
                        icon="plus"
                        size="sm"
                    />
                </template>
                {{ t('system.email_templates.create') }}
            </v-btn>
        </div>

        <v-data-table
            :items="templates"
            :headers="headers"
            :loading="loading"
            :no-data-text="t('table.no_data')"
            :loading-text="t('table.loading')"
        >
            <template #item.template_type="{ item }">
                <v-chip
                    :color="item.template_type === 'system' ? 'warning' : 'primary'"
                    size="small"
                    variant="tonal"
                >
                    {{ item.template_type }}
                </v-chip>
            </template>

            <template #item.updated_at="{ item }">
                <span class="text-body-2">{{ formatDate(item.updated_at) }}</span>
            </template>

            <template #item.actions="{ item }">
                <div class="d-flex gap-2">
                    <v-btn
                        variant="text"
                        size="small"
                        color="info"
                        data-testid="edit-template-btn"
                        @click="openEdit(item)"
                    >
                        <SmartIcon
                            icon="pencil"
                            size="sm"
                        />
                    </v-btn>
                    <v-btn
                        variant="text"
                        size="small"
                        color="error"
                        :disabled="item.template_type === 'system'"
                        @click="confirmDelete(item)"
                    >
                        <SmartIcon
                            icon="trash-2"
                            size="sm"
                        />
                    </v-btn>
                </div>
            </template>
        </v-data-table>

        <EmailTemplateEditor
            v-model="showEditor"
            :template="editingTemplate"
            @saved="load"
        />

        <DialogManager
            v-model="showDeleteDialog"
            :config="deleteDialogConfig"
            :loading="deleting"
            @confirm="performDelete"
        >
            <p>{{ t('system.email_templates.confirm_delete') }}</p>
        </DialogManager>

        <SnackbarManager :snackbar="currentSnackbar" />
    </div>
</template>

<script setup lang="ts">
    import { ref, computed, onMounted } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import { useSnackbar } from '@/composables/useSnackbar'
    import { useErrorHandler } from '@/composables/useErrorHandler'
    import { typedHttpClient } from '@/api/typed-client'
    import type { EmailTemplate } from '@/api/clients/email-templates'
    import EmailTemplateEditor from './EmailTemplateEditor.vue'
    import DialogManager from '@/components/common/DialogManager.vue'
    import SnackbarManager from '@/components/common/SnackbarManager.vue'
    import SmartIcon from '@/components/common/SmartIcon.vue'

    const { t } = useTranslations()
    const { currentSnackbar, showSuccess } = useSnackbar()
    const { handleError } = useErrorHandler()

    const templates = ref<EmailTemplate[]>([])
    const loading = ref(false)
    const showEditor = ref(false)
    const editingTemplate = ref<EmailTemplate | null>(null)
    const showDeleteDialog = ref(false)
    const templateToDelete = ref<EmailTemplate | null>(null)
    const deleting = ref(false)

    const headers = computed(() => [
        { title: t('system.email_templates.name'), key: 'name', sortable: true },
        { title: t('system.email_templates.slug'), key: 'slug', sortable: true },
        { title: t('system.email_templates.type'), key: 'template_type', sortable: true },
        { title: t('entities.table.updated_at'), key: 'updated_at', sortable: true },
        { title: '', key: 'actions', sortable: false },
    ])

    const deleteDialogConfig = computed(() => ({
        title: t('system.email_templates.delete'),
        confirmText: t('system.email_templates.delete'),
        cancelText: t('common.cancel'),
        maxWidth: '400px',
    }))

    const load = async () => {
        loading.value = true
        try {
            templates.value = await typedHttpClient.listEmailTemplates()
        } catch (err) {
            handleError(err)
        } finally {
            loading.value = false
        }
    }

    const openCreate = () => {
        editingTemplate.value = null
        showEditor.value = true
    }

    const openEdit = (template: EmailTemplate) => {
        editingTemplate.value = template
        showEditor.value = true
    }

    const confirmDelete = (template: EmailTemplate) => {
        templateToDelete.value = template
        showDeleteDialog.value = true
    }

    const performDelete = async () => {
        if (!templateToDelete.value) return
        deleting.value = true
        try {
            await typedHttpClient.deleteEmailTemplate(templateToDelete.value.uuid)
            showSuccess(t('system.email_templates.deleted'))
            showDeleteDialog.value = false
            templateToDelete.value = null
            await load()
        } catch (err) {
            handleError(err)
        } finally {
            deleting.value = false
        }
    }

    const formatDate = (dateString: string): string => {
        try {
            return new Date(dateString).toLocaleString()
        } catch {
            return dateString
        }
    }

    onMounted(() => {
        void load()
    })
</script>
