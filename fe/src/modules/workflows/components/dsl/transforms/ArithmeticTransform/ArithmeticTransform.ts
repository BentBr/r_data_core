import { ref, watch, computed, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import type { Transform, Operand } from '../../contracts'

export default defineComponent({
    name: 'ArithmeticTransform',
    components: {
        SmartIcon,
    },
    props: {
        modelValue: {
            type: Object as PropType<Transform>,
            required: true,
        },
        availableFields: {
            type: Array as PropType<string[]>,
            default: () => [],
        },
    },
    emits: ['update:modelValue'],
    setup(props, { emit }) {
        const { t } = useTranslations()

        const ops = computed(() => [
            { title: t('workflows.dsl.op_add'), value: 'add', icon: 'plus' },
            { title: t('workflows.dsl.op_sub'), value: 'sub', icon: 'minus' },
            { title: t('workflows.dsl.op_mul'), value: 'mul', icon: 'x' },
            { title: t('workflows.dsl.op_div'), value: 'div', icon: 'divide' },
        ])

        const operandKinds = ['field', 'const']

        const arithmeticTarget = computed(() => {
            if (props.modelValue.type === 'arithmetic') {
                return props.modelValue.target
            }
            return ''
        })

        const arithmeticOp = computed(() => {
            if (props.modelValue.type === 'arithmetic') {
                return props.modelValue.op
            }
            return 'add'
        })

        const left = ref<Operand>({ kind: 'field', field: '' })
        const right = ref<Operand>({ kind: 'const', value: 0 })

        watch(
            () => props.modelValue,
            newTransform => {
                if (newTransform.type === 'arithmetic') {
                    left.value = newTransform.left
                    right.value = newTransform.right
                }
            },
            { immediate: true, deep: true }
        )

        function updateField(field: string, value: unknown) {
            if (props.modelValue.type === 'arithmetic') {
                const updated: Transform = {
                    ...props.modelValue,
                    [field]: value,
                }
                emit('update:modelValue', updated)
            }
        }

        function updateLeftKind(kind: 'field' | 'const' | 'external_entity_field') {
            if (kind === 'field' || kind === 'external_entity_field') {
                left.value = { kind: 'field', field: '' }
            } else {
                left.value = { kind: 'const', value: 0 }
            }
            syncLeft()
        }

        function updateLeftField(field: string) {
            left.value = { kind: 'field', field }
            syncLeft()
        }

        function updateLeftValue(value: number) {
            left.value = { kind: 'const', value }
            syncLeft()
        }

        function updateRightKind(kind: 'field' | 'const' | 'external_entity_field') {
            if (kind === 'field' || kind === 'external_entity_field') {
                right.value = { kind: 'field', field: '' }
            } else {
                right.value = { kind: 'const', value: 0 }
            }
            syncRight()
        }

        function updateRightField(field: string) {
            right.value = { kind: 'field', field }
            syncRight()
        }

        function updateRightValue(value: number) {
            right.value = { kind: 'const', value }
            syncRight()
        }

        function syncLeft() {
            if (props.modelValue.type === 'arithmetic') {
                const updated: Transform = {
                    ...props.modelValue,
                    left: { ...left.value },
                }
                emit('update:modelValue', updated)
            }
        }

        function syncRight() {
            if (props.modelValue.type === 'arithmetic') {
                const updated: Transform = {
                    ...props.modelValue,
                    right: { ...right.value },
                }
                emit('update:modelValue', updated)
            }
        }

        return {
            t,
            ops,
            operandKinds,
            arithmeticTarget,
            arithmeticOp,
            left,
            right,
            updateField,
            updateLeftKind,
            updateLeftField,
            updateLeftValue,
            updateRightKind,
            updateRightField,
            updateRightValue,
        }
    },
})
