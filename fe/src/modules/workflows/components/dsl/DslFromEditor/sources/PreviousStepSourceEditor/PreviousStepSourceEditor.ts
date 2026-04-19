import { ref, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import type { FromDef } from '../../../contracts'
import MappingEditor from '../../../MappingEditor/index.vue'

export default defineComponent({
    name: 'PreviousStepSourceEditor',
    components: {
        MappingEditor,
    },
    props: {
        modelValue: { type: Object as PropType<FromDef>, required: true },
        stepIndex: { type: Number, required: true },
        previousStepFields: { type: Array as PropType<string[]>, default: () => [] },
    },
    emits: ['update:modelValue'],
    setup(props, { emit }) {
        const { t } = useTranslations()
        const mappingEditorRef = ref<any>(null)

        function updateMapping(mapping: any) {
            emit('update:modelValue', { ...props.modelValue, mapping } as FromDef)
        }

        function addMapping() { mappingEditorRef.value?.addEmptyPair() }

        return { t, mappingEditorRef, updateMapping, addMapping }
    }
})
