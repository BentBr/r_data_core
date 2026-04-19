import { computed, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import { buildApiUrl } from '@/env-check'
import type { ToDef, AuthConfig, HttpMethod, OutputMode } from '../../../contracts'
import { defaultCsvOptions } from '../../../dsl-utils'
import CsvOptionsEditor from '../../../CsvOptionsEditor/index.vue'
import MappingEditor from '../../../MappingEditor/index.vue'
import AuthConfigEditor from '../../../AuthConfigEditor/index.vue'

export default defineComponent({
    name: 'FormatOutputEditor',
    components: {
        CsvOptionsEditor,
        MappingEditor,
        AuthConfigEditor,
    },
    props: {
        modelValue: { type: Object as PropType<ToDef>, required: true },
        workflowUuid: { type: String, default: null },
    },
    emits: ['update:modelValue'],
    setup(props, { emit }) {
        const { t } = useTranslations()

        const formatType = computed(() => props.modelValue.type === 'format' ? props.modelValue.format.format_type : 'json')
        const formatOptions = computed(() => props.modelValue.type === 'format' ? props.modelValue.format.options ?? {} : {})
        
        const outputMode = computed(() => {
            if (props.modelValue.type !== 'format') return 'api'
            const output = props.modelValue.output
            return typeof output === 'string' ? output : output.mode
        })

        const outputDestinationType = computed(() => {
            if (props.modelValue.type === 'format') {
                const output = props.modelValue.output
                if (typeof output === 'object' && output.mode === 'push') {
                    return output.destination.destination_type
                }
            }
            return 'uri'
        })

        const outputMethod = computed(() => {
            if (props.modelValue.type === 'format') {
                const output = props.modelValue.output
                if (typeof output === 'object' && output.mode === 'push') {
                    return output.method ?? 'POST'
                }
            }
            return 'POST'
        })

        const outputDestinationUri = computed(() => {
            if (props.modelValue.type === 'format') {
                const output = props.modelValue.output
                if (typeof output === 'object' && output.mode === 'push' && 'uri' in output.destination.config) {
                    return String(output.destination.config.uri)
                }
            }
            return ''
        })

        const outputDestinationAuth = computed((): AuthConfig => {
            if (props.modelValue.type === 'format') {
                const output = props.modelValue.output
                if (typeof output === 'object' && output.mode === 'push') {
                    return output.destination.auth ?? { type: 'none' }
                }
            }
            return { type: 'none' }
        })

        const outputModes = [
            { title: 'API', value: 'api' },
            { title: 'Download', value: 'download' },
            { title: 'Push', value: 'push' },
        ]
        const formatTypes = [
            { title: 'CSV', value: 'csv' },
            { title: 'JSON', value: 'json' },
        ]
        const destinationTypes = [{ title: 'URI', value: 'uri' }]
        const httpMethods: { title: string; value: HttpMethod }[] = [
            { title: 'GET', value: 'GET' },
            { title: 'POST', value: 'POST' },
            { title: 'PUT', value: 'PUT' },
            { title: 'PATCH', value: 'PATCH' },
            { title: 'DELETE', value: 'DELETE' },
        ]

        function updateField(field: string, value: any) {
            emit('update:modelValue', { ...props.modelValue, [field]: value } as ToDef)
        }

        function updateFormatType(newType: string) {
            if (props.modelValue.type !== 'format') return
            const updated = { ...props.modelValue, format: { ...props.modelValue.format, format_type: newType, options: newType === 'csv' ? defaultCsvOptions() : {} } }
            emit('update:modelValue', updated as ToDef)
        }

        function updateOutputMode(mode: string) {
            if (props.modelValue.type !== 'format') return
            let output: OutputMode
            if (mode === 'push') {
                output = { mode: 'push', destination: { destination_type: 'uri', config: { uri: '' }, auth: { type: 'none' } }, method: 'POST' }
            } else if (mode === 'download') {
                output = { mode: 'download' }
            } else {
                output = { mode: 'api' }
            }
            emit('update:modelValue', { ...props.modelValue, output } as ToDef)
        }

        function updateDestinationConfig(key: string, value: any) {
            if (props.modelValue.type !== 'format') return
            const currentOutput = props.modelValue.output
            if (typeof currentOutput === 'object' && currentOutput.mode === 'push') {
                const updated = { ...props.modelValue, output: { ...currentOutput, destination: { ...currentOutput.destination, config: { ...currentOutput.destination.config, [key]: value } } } }
                emit('update:modelValue', updated as ToDef)
            }
        }

        function updateHttpMethod(method: HttpMethod) {
            if (props.modelValue.type !== 'format') return
            const currentOutput = props.modelValue.output
            if (typeof currentOutput === 'object' && currentOutput.mode === 'push') {
                emit('update:modelValue', { ...props.modelValue, output: { ...currentOutput, method } } as ToDef)
            }
        }

        return {
            t, formatType, formatOptions, outputMode, outputDestinationType, outputMethod, outputDestinationUri, outputDestinationAuth,
            outputModes, formatTypes, destinationTypes, httpMethods,
            updateField, updateFormatType, updateOutputMode, updateDestinationConfig, updateHttpMethod,
            getFullEndpointUri: () => buildApiUrl(`/api/v1/workflows/${props.workflowUuid ?? '{uuid}'}`)
        }
    }
})
