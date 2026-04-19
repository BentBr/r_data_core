import { ref, watch, defineComponent, PropType } from 'vue'
import { buttonConfigs } from '@/design-system/components'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'

type Pair = { k: string; v: string }

export default defineComponent({
    name: 'MappingTable',
    components: {
        SmartIcon,
    },
    props: {
        pairs: { type: Array as PropType<Pair[]>, required: true },
        leftLabel: { type: String, required: true },
        rightLabel: { type: String, required: true },
        rightItems: { type: Array as PropType<string[]>, default: undefined },
        useSelectForRight: { type: Boolean, default: false },
    },
    emits: ['update-pair', 'delete-pair'],
    setup(props, { emit }) {
        const localPairs = ref<Pair[]>([])
        watch(() => props.pairs, v => {
            localPairs.value = Array.isArray(v) ? v.map(p => ({ ...p })) : []
        }, { immediate: true, deep: true })
        function emitUpdate(idx: number) {
            const pair = localPairs.value[idx]
            emit('update-pair', idx, { ...pair })
        }
        return { localPairs, buttonConfigs, emitUpdate, emit }
    },
})
