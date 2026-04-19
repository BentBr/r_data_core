import { ref, watch, nextTick, defineComponent, PropType } from 'vue'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import type { Mapping } from '../dsl-utils'
import { getMappingPairs, pairsToMapping } from '../dsl-utils'

type Pair = { k: string; v: string }

export default defineComponent({
    name: 'MappingEditor',
    components: {
        SmartIcon,
    },
    props: {
        modelValue: { type: Object as PropType<Mapping>, required: true },
        leftLabel: { type: String, required: true },
        rightLabel: { type: String, required: true },
        leftItems: { type: Array as PropType<string[]>, default: undefined },
        useSelectForLeft: { type: Boolean, default: false },
        rightItems: { type: Array as PropType<string[]>, default: undefined },
        useSelectForRight: { type: Boolean, default: false },
    },
    emits: ['update:modelValue', 'add-pair'],
    setup(props, { emit }) {
        const localPairs = ref<Pair[]>([])
        let isUpdatingFromLocal = false

        function addEmptyPair() { localPairs.value.push({ k: '', v: '' }) }

        watch(() => props.modelValue, newMapping => {
            if (isUpdatingFromLocal) return
            const newPairs = getMappingPairs(newMapping)
            const currentEmptyPairs = localPairs.value.filter(p => !p.k && !p.v)
            if (JSON.stringify(localPairs.value.filter(p => p.k || p.v)) !== JSON.stringify(newPairs.filter(p => p.k || p.v))) {
                localPairs.value = [...newPairs, ...currentEmptyPairs].map(p => ({ ...p }))
            }
        }, { immediate: true })

        function updatePair(idx: number, pair: Pair) {
            localPairs.value[idx] = { ...pair }
            isUpdatingFromLocal = true
            emit('update:modelValue', pairsToMapping(localPairs.value))
            void nextTick(() => { isUpdatingFromLocal = false })
        }

        function deletePair(idx: number) {
            localPairs.value.splice(idx, 1)
            void nextTick(() => { emit('update:modelValue', pairsToMapping(localPairs.value)) })
        }

        return { localPairs, addEmptyPair, updatePair, deletePair }
    },
})
