import { defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import type { ToDef } from '../contracts'
import FormatOutputEditor from './outputs/FormatOutputEditor/index.vue'
import EntityOutputEditor from './outputs/EntityOutputEditor/index.vue'
import NextStepOutputEditor from './outputs/NextStepOutputEditor/index.vue'

export default defineComponent({
    name: 'DslToEditor',
    components: {
        FormatOutputEditor,
        EntityOutputEditor,
        NextStepOutputEditor,
    },
    props: {
        modelValue: { type: Object as PropType<ToDef>, required: true },
        workflowUuid: { type: String as PropType<string | null>, default: null },
        isLastStep: { type: Boolean, default: false },
    },
    emits: ['update:modelValue'],
    setup(_props, { emit }) {
        const { t } = useTranslations()

        const toTypes = [
            { title: 'Format (CSV/JSON)', value: 'format' },
            { title: 'Entity', value: 'entity' },
            { title: 'Next Step', value: 'next_step' },
        ]

        function onTypeChange(newType: 'format' | 'entity' | 'next_step') {
            let newTo: ToDef
            if (newType === 'format') {
                newTo = {
                    type: 'format',
                    output: { mode: 'api' },
                    format: { format_type: 'json', options: {} },
                    mapping: {},
                }
            } else if (newType === 'entity') {
                newTo = { type: 'entity', entity_definition: '', mode: 'create', mapping: {} }
            } else {
                newTo = { type: 'next_step', mapping: {} }
            }
            emit('update:modelValue', newTo)
        }

        return { t, toTypes, onTypeChange, emit }
    },
})
