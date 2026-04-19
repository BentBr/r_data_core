import { ref, computed, watch, onMounted, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import { useEntityDefinitions } from '@/shared/composables/useEntityDefinitions'
import { typedHttpClient } from '@/api/typed-client'
import type { FromDef } from '../../../contracts'
import MappingEditor from '../../../MappingEditor/index.vue'

export default defineComponent({
    name: 'EntitySourceEditor',
    components: {
        MappingEditor,
    },
    props: {
        modelValue: { type: Object as PropType<FromDef>, required: true },
    },
    emits: ['update:modelValue'],
    setup(props, { emit }) {
        const { t } = useTranslations()
        const { entityDefinitions, loadEntityDefinitions } = useEntityDefinitions()

        const mappingEditorRef = ref<any>(null)
        const entityDefItems = ref<{ title: string; value: string }[]>([])
        const filterFieldItems = ref<string[]>([])

        const entityDefinition = computed(() => props.modelValue.type === 'entity' ? props.modelValue.entity_definition : '')
        const filterField = computed(() => props.modelValue.type === 'entity' ? props.modelValue.filter?.field ?? '' : '')
        const filterValue = computed(() => props.modelValue.type === 'entity' ? props.modelValue.filter?.value ?? '' : '')
        const filterOperator = computed(() => props.modelValue.type === 'entity' ? props.modelValue.filter?.operator ?? '=' : '=')
        const filterEnabled = computed(() => props.modelValue.type === 'entity' ? !!props.modelValue.filter : false)

        watch(() => entityDefinitions.value, (defs) => {
            entityDefItems.value = defs.map(d => ({ title: d.display_name || d.entity_type, value: d.entity_type }))
        }, { immediate: true })

        onMounted(() => { void loadEntityDefinitions() })

        async function loadEntityFields(entityType: string) {
            if (!entityType) return
            try {
                const fields = await typedHttpClient.getEntityFields(entityType)
                filterFieldItems.value = fields.map(f => f.name)
            } catch { filterFieldItems.value = [] }
        }

        watch(() => entityDefinition.value, (newVal) => { if (newVal) void loadEntityFields(newVal) }, { immediate: true })

        function updateField(field: string, value: any) {
            emit('update:modelValue', { ...props.modelValue, [field]: value } as FromDef)
        }

        function updateFilterField(field: string, value: any) {
            if (props.modelValue.type !== 'entity') return
            const updated = { ...props.modelValue, filter: { ...props.modelValue.filter, [field]: value } }
            emit('update:modelValue', updated as FromDef)
        }

        function toggleFilter(enabled: boolean | null) {
            if (props.modelValue.type !== 'entity') return
            const updated = { ...props.modelValue } as any
            if (enabled) updated.filter = { field: '', operator: '=', value: '' }
            else delete updated.filter
            emit('update:modelValue', updated as FromDef)
        }

        function updateMapping(mapping: any) {
            emit('update:modelValue', { ...props.modelValue, mapping } as FromDef)
        }

        function addMapping() { mappingEditorRef.value?.addEmptyPair() }

        return {
            t, entityDefItems, filterFieldItems, entityDefinition, filterField, filterValue, filterOperator, filterEnabled, mappingEditorRef,
            updateField, updateFilterField, toggleFilter, updateMapping, addMapping,
        }
    }
})
