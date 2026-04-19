import { computed, defineComponent, PropType } from 'vue'
import type { DslStep, FromDef, Transform, ToDef } from '../contracts'
import { buildStepSummary } from '../summary'
import { useTranslations } from '@/shared/composables/useTranslations'
import DslFromEditor from '../DslFromEditor/index.vue'
import DslTransformEditor from '../DslTransformEditor/index.vue'
import DslToEditor from '../DslToEditor/index.vue'

export default defineComponent({
    name: 'DslStepEditor',
    components: {
        DslFromEditor,
        DslTransformEditor,
        DslToEditor,
    },
    props: {
        modelValue: {
            type: Object as PropType<DslStep>,
            required: true,
        },
        workflowUuid: {
            type: String as PropType<string | null>,
            default: null,
        },
        stepIndex: {
            type: Number,
            default: 0,
        },
        previousStepFields: {
            type: Array as PropType<string[]>,
            default: () => [],
        },
        normalizedFields: {
            type: Array as PropType<string[]>,
            default: () => [],
        },
        isLastStep: {
            type: Boolean,
            default: false,
        },
    },
    emits: ['update:modelValue'],
    setup(props, { emit }) {
        const { t } = useTranslations()

        function updateFrom(from: FromDef) {
            emit('update:modelValue', { ...props.modelValue, from })
        }

        function updateTransform(transform: Transform) {
            emit('update:modelValue', { ...props.modelValue, transform })
        }

        function updateTo(to: ToDef) {
            emit('update:modelValue', { ...props.modelValue, to })
        }

        const stepSummary = computed(() => buildStepSummary(props.modelValue, t))

        return {
            t,
            updateFrom,
            updateTransform,
            updateTo,
            stepSummary,
        }
    },
})
