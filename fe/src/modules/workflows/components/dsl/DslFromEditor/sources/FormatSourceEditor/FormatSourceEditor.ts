import { ref, computed, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import { buildApiUrl } from '@/env-check'
import type { FromDef, AuthConfig } from '../../../contracts'
import { defaultCsvOptions } from '../../../dsl-utils'
import CsvOptionsEditor from '../../../CsvOptionsEditor/index.vue'
import MappingEditor from '../../../MappingEditor/index.vue'
import AuthConfigEditor from '../../../AuthConfigEditor/index.vue'

export default defineComponent({
    name: 'FormatSourceEditor',
    components: {
        CsvOptionsEditor,
        MappingEditor,
        AuthConfigEditor,
    },
    props: {
        modelValue: { type: Object as PropType<FromDef>, required: true },
        workflowUuid: { type: String, default: null },
    },
    emits: ['update:modelValue'],
    setup(props, { emit }) {
        const { t } = useTranslations()
        const mappingEditorRef = ref<any>(null)

        const sourceType = computed(() => props.modelValue.type === 'format' ? props.modelValue.source.source_type : 'uri')
        const formatType = computed(() => props.modelValue.type === 'format' ? props.modelValue.format.format_type : 'csv')
        const sourceUri = computed(() => {
            if (props.modelValue.type !== 'format') return ''
            const config = props.modelValue.source.config
            return (config as any).uri || ''
        })
        const formatOptions = computed(() => props.modelValue.type === 'format' ? props.modelValue.format.options ?? {} : {})
        const sourceAuth = computed(() => props.modelValue.type === 'format' ? props.modelValue.source.auth ?? { type: 'none' } : { type: 'none' })

        const sourceTypes = [
            { title: 'URI', value: 'uri' },
            { title: 'API', value: 'api' },
            { title: 'File', value: 'file' },
        ]
        const formatTypes = [
            { title: 'CSV', value: 'csv' },
            { title: 'JSON', value: 'json' },
        ]

        function updateSourceType(newType: string) {
            if (props.modelValue.type !== 'format') return
            const updated = { ...props.modelValue, source: { ...props.modelValue.source, source_type: newType, config: newType === 'uri' ? { uri: '' } : {} } }
            emit('update:modelValue', updated as FromDef)
        }

        function updateFormatType(newType: string) {
            if (props.modelValue.type !== 'format') return
            const updated = { ...props.modelValue, format: { ...props.modelValue.format, format_type: newType, options: newType === 'csv' ? defaultCsvOptions() : {} } }
            emit('update:modelValue', updated as FromDef)
        }

        function updateSourceConfig(key: string, value: any) {
            if (props.modelValue.type !== 'format') return
            const updated = { ...props.modelValue, source: { ...props.modelValue.source, config: { ...props.modelValue.source.config, [key]: value } } }
            emit('update:modelValue', updated as FromDef)
        }

        function updateFormatOptions(options: any) {
            if (props.modelValue.type !== 'format') return
            const updated = { ...props.modelValue, format: { ...props.modelValue.format, options } }
            emit('update:modelValue', updated as FromDef)
        }

        function updateSourceAuth(auth: AuthConfig) {
            if (props.modelValue.type !== 'format') return
            const updated = { ...props.modelValue, source: { ...props.modelValue.source, auth } }
            emit('update:modelValue', updated as FromDef)
        }

        function updateMapping(mapping: any) {
            emit('update:modelValue', { ...props.modelValue, mapping } as FromDef)
        }

        function addMapping() { mappingEditorRef.value?.addEmptyPair() }

        async function onTestUpload(e: Event) {
            const input = e.target as HTMLInputElement; const file = input.files?.[0]
            if (!file || props.modelValue.type !== 'format' || props.modelValue.format.format_type !== 'csv') return
            const text = await file.text(); const lines = text.split('\n'); if (!lines.length) return
            const header = lines[0].split(',').map(s => s.trim()); const mapping: Record<string, string> = {}
            for (const f of header) if (f) mapping[f] = f
            updateMapping(mapping)
        }

        async function autoMapFromUri() {
            if (props.modelValue.type !== 'format' || !sourceUri.value) return
            try {
                const res = await fetch(sourceUri.value); const txt = await res.text(); const lines = txt.split('\n'); if (!lines.length) return
                const header = lines[0].split(',').map(s => s.trim()); const mapping: Record<string, string> = {}
                for (const f of header) if (f) mapping[f] = f
                updateMapping(mapping)
            } catch (e) { /* ignore */ }
        }

        return {
            t, sourceType, formatType, sourceUri, formatOptions, sourceAuth, sourceTypes, formatTypes, mappingEditorRef,
            updateSourceType, updateFormatType, updateSourceConfig, updateFormatOptions, updateSourceAuth, updateMapping, addMapping,
            onTestUpload, autoMapFromUri, getFullEndpointUri: () => buildApiUrl(`/api/v1/workflows/${props.workflowUuid ?? '{uuid}'}`)
        }
    }
})
