import { ref, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import type { ToDef } from '../../../contracts'
import MappingEditor from '../../../MappingEditor/index.vue'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'

export default defineComponent({
    name: 'NextStepOutputEditor',
    components: {
        MappingEditor,
        SmartIcon,
    },
    props: {
        modelValue: { type: Object as PropType<ToDef>, required: true },
        isLastStep: { type: Boolean, default: false },
    },
    emits: ['update:modelValue'],
    setup(props, { emit }) {
        const { t } = useTranslations()
        const mappingEditorRef = ref<any>(null)

        function updateMapping(mapping: any) {
            emit('update:modelValue', { ...props.modelValue, mapping } as ToDef)
        }

        return {
            t, mappingEditorRef, updateMapping,
            addMapping: () => mappingEditorRef.value?.addEmptyPair()
        }
    }
})
