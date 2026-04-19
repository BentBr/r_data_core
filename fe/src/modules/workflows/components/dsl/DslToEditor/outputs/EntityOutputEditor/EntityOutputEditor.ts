import { ref, computed, watch, onMounted, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import { useEntityDefinitions } from '@/shared/composables/useEntityDefinitions'
import { typedHttpClient } from '@/api/typed-client'
import type { ToDef } from '../../../contracts'
import MappingEditor from '../../../MappingEditor/index.vue'

export default defineComponent({
    name: 'EntityOutputEditor',
    components: {
        MappingEditor,
    },
    props: {
        modelValue: { type: Object as PropType<ToDef>, required: true },
    },
    emits: ['update:modelValue'],
    setup(props, { emit }) {
        const { t } = useTranslations()
        const { entityDefinitions, loadEntityDefinitions } = useEntityDefinitions()

        const mappingEditorRef = ref<any>(null)
        const entityDefItems = ref<{ title: string; value: string }[]>([])
        const entityTargetFields = ref<string[]>([])

        const entityDefinition = computed(() => props.modelValue.type === 'entity' ? props.modelValue.entity_definition : '')
        const entityPath = computed(() => props.modelValue.type === 'entity' ? props.modelValue.path : '')
        const entityMode = computed(() => props.modelValue.type === 'entity' ? props.modelValue.mode : 'create')
        const entityUpdateKey = computed(() => props.modelValue.type === 'entity' ? props.modelValue.update_key ?? '' : '')

        const entityModes = [
            { title: 'Create', value: 'create' },
            { title: 'Update', value: 'update' },
            { title: 'Create or Update', value: 'create_or_update' },
        ]

        watch(() => entityDefinitions.value, (defs) => {
            entityDefItems.value = defs.map(d => ({ title: d.display_name || d.entity_type, value: d.entity_type }))
        }, { immediate: true })

        onMounted(() => { void loadEntityDefinitions() })

        async function loadEntityFields(entityType: string) {
            if (!entityType) return
            try {
                const fields = await typedHttpClient.getEntityFields(entityType)
                entityTargetFields.value = fields.map(f => f.name).filter(n => !['uuid', 'created_at', 'updated_at'].includes(n))
            } catch { entityTargetFields.value = [] }
        }

        watch(() => entityDefinition.value, (newVal) => { if (newVal) void loadEntityFields(newVal) }, { immediate: true })

        function updateField(field: string, value: any) {
            emit('update:modelValue', { ...props.modelValue, [field]: value } as ToDef)
        }

        return {
            t, entityDefItems, entityTargetFields, entityDefinition, entityPath, entityMode, entityUpdateKey, entityModes, mappingEditorRef,
            updateField,
            addMapping: () => mappingEditorRef.value?.addEmptyPair()
        }
    }
})
