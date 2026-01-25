<template>
    <v-dialog
        v-model="showDialog"
        :max-width="getDialogMaxWidth('form')"
    >
        <v-card>
            <v-card-title class="text-h5 pa-6">
                {{ t('entity_definitions.create.title') }}
            </v-card-title>
            <v-card-text class="pa-6">
                <v-form
                    ref="formRef"
                    v-model="formValid"
                >
                    <v-row>
                        <v-col cols="6">
                            <v-text-field
                                v-model="form.entity_type"
                                :label="t('entity_definitions.create.entity_type_label')"
                                :hint="t('entity_definitions.create.entity_type_hint')"
                                :rules="[
                                    v => !!v ?? t('entity_definitions.create.entity_type_required'),
                                ]"
                                required
                            />
                        </v-col>
                        <v-col cols="6">
                            <v-text-field
                                v-model="form.display_name"
                                :label="t('entity_definitions.create.display_name_label')"
                                :rules="[
                                    v =>
                                        !!v ?? t('entity_definitions.create.display_name_required'),
                                ]"
                                required
                            />
                        </v-col>
                    </v-row>
                    <v-row>
                        <v-col cols="12">
                            <v-textarea
                                v-model="form.description"
                                :label="t('entity_definitions.create.description_label')"
                                rows="3"
                            />
                        </v-col>
                    </v-row>
                    <v-row>
                        <v-col cols="12">
                            <v-text-field
                                v-model="form.group_name"
                                :label="t('entity_definitions.create.group_name_label')"
                            />
                        </v-col>
                    </v-row>
                    <v-row>
                        <v-col cols="12">
                            <IconPicker
                                v-model="form.icon"
                                :label="t('entity_definitions.create.icon_label')"
                            />
                        </v-col>
                    </v-row>
                    <v-row>
                        <v-col cols="6">
                            <v-switch
                                v-model="form.allow_children"
                                :label="t('entity_definitions.create.allow_children_label')"
                                color="primary"
                            />
                        </v-col>
                        <v-col cols="6">
                            <v-switch
                                v-model="form.published"
                                :label="t('entity_definitions.create.published_label')"
                                color="success"
                            />
                        </v-col>
                    </v-row>
                </v-form>
            </v-card-text>
            <v-card-actions class="pa-4 px-6">
                <v-spacer />
                <v-btn
                    variant="text"
                    color="mutedForeground"
                    @click="closeDialog"
                >
                    {{ t('common.cancel') }}
                </v-btn>
                <v-btn
                    color="primary"
                    variant="flat"
                    :loading="loading"
                    :disabled="!formValid"
                    @click="createEntityDefinition"
                >
                    {{ t('entity_definitions.create.create_button') }}
                </v-btn>
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>

<script setup lang="ts">
    import { ref, computed } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import IconPicker from '@/components/common/IconPicker.vue'
    import { getDialogMaxWidth } from '@/design-system/components'
    import type { CreateEntityDefinitionRequest } from '@/types/schemas'

    interface Props {
        modelValue: boolean
        loading?: boolean
    }

    interface Emits {
        (e: 'update:modelValue', value: boolean): void

        (e: 'create', data: CreateEntityDefinitionRequest): void
    }

    const props = withDefaults(defineProps<Props>(), {
        loading: false,
    })

    const emit = defineEmits<Emits>()
    const { t } = useTranslations()

    const showDialog = computed({
        get: () => props.modelValue,
        set: value => emit('update:modelValue', value),
    })

    const formValid = ref(false)
    const formRef = ref<HTMLFormElement | null>(null)

    const form = ref<CreateEntityDefinitionRequest>({
        entity_type: '',
        display_name: '',
        description: '',
        group_name: '',
        allow_children: false,
        icon: '',
        fields: [],
        published: false,
    })

    const closeDialog = () => {
        showDialog.value = false
        resetForm()
    }

    const resetForm = () => {
        form.value = {
            entity_type: '',
            display_name: '',
            description: '',
            group_name: '',
            allow_children: false,
            icon: '',
            fields: [],
            published: false,
        }
    }

    const createEntityDefinition = () => {
        if (!formValid.value) {
            return
        }

        emit('create', { ...form.value })
        closeDialog()
    }
</script>
