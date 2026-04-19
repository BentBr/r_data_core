import { defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import type { FromDef } from '../contracts'
import { defaultCsvOptions } from '../dsl-utils'
import FormatSourceEditor from './sources/FormatSourceEditor/index.vue'
import EntitySourceEditor from './sources/EntitySourceEditor/index.vue'
import TriggerSourceEditor from './sources/TriggerSourceEditor/index.vue'
import PreviousStepSourceEditor from './sources/PreviousStepSourceEditor/index.vue'

export default defineComponent({
    name: 'DslFromEditor',
    components: {
        FormatSourceEditor,
        EntitySourceEditor,
        TriggerSourceEditor,
        PreviousStepSourceEditor,
    },
    props: {
        modelValue: { type: Object as PropType<FromDef>, required: true },
        workflowUuid: { type: String as PropType<string | null>, default: null },
        stepIndex: { type: Number, default: 0 },
        previousStepFields: { type: Array as PropType<string[]>, default: () => [] },
    },
    emits: ['update:modelValue'],
    setup(_props, { emit }) {
        const { t } = useTranslations()

        const fromTypes = [
            { title: 'Format (CSV/JSON)', value: 'format' },
            { title: 'Entity', value: 'entity' },
            { title: 'Previous Step', value: 'previous_step' },
            { title: 'Trigger', value: 'trigger' },
        ]

        function onTypeChange(newType: 'format' | 'entity' | 'previous_step' | 'trigger') {
            let newFrom: FromDef
            if (newType === 'format') {
                newFrom = {
                    type: 'format',
                    source: { source_type: 'uri', config: { uri: '' }, auth: { type: 'none' } },
                    format: { format_type: 'csv', options: defaultCsvOptions() },
                    mapping: {},
                }
            } else if (newType === 'entity') {
                newFrom = { type: 'entity', entity_definition: '', filter: { field: '', operator: '=', value: '' }, mapping: {} }
            } else if (newType === 'trigger') {
                newFrom = { type: 'trigger', mapping: {} }
            } else {
                newFrom = { type: 'previous_step', mapping: {} }
            }
            emit('update:modelValue', newFrom)
        }

        return { t, fromTypes, onTypeChange, emit }
    },
})
