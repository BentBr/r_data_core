import { ref, watch, computed, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import type { Transform, StringOperand } from '../../contracts'

export default defineComponent({
    name: 'ConcatTransform',
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

        const stringOperandKinds = ['field', 'const_string']

        const concatSeparator = computed(() => {
            if (props.modelValue.type === 'concat') {
                return props.modelValue.separator ?? ' '
            }
            return ' '
        })

        const leftConcat = ref<StringOperand>({ kind: 'field', field: '' })
        const rightConcat = ref<StringOperand>({ kind: 'const_string', value: '' })

        watch(
            () => props.modelValue,
            newTransform => {
                if (newTransform.type === 'concat') {
                    leftConcat.value = newTransform.left
                    rightConcat.value = newTransform.right
                }
            },
            { immediate: true, deep: true }
        )

        function updateField(field: string, value: unknown) {
            if (props.modelValue.type === 'concat') {
                const updated: Transform = {
                    ...props.modelValue,
                    [field]: value,
                }
                emit('update:modelValue', updated)
            }
        }

        function updateLeftConcatKind(kind: 'field' | 'const_string') {
            if (kind === 'field') {
                leftConcat.value = { kind: 'field', field: '' }
            } else {
                leftConcat.value = { kind: 'const_string', value: '' }
            }
            syncLeftConcat()
        }

        function updateLeftConcatField(field: string) {
            leftConcat.value = { kind: 'field', field }
            syncLeftConcat()
        }

        function updateLeftConcatValue(value: string) {
            leftConcat.value = { kind: 'const_string', value }
            syncLeftConcat()
        }

        function updateRightConcatKind(kind: 'field' | 'const_string') {
            if (kind === 'field') {
                rightConcat.value = { kind: 'field', field: '' }
            } else {
                rightConcat.value = { kind: 'const_string', value: '' }
            }
            syncRightConcat()
        }

        function updateRightConcatField(field: string) {
            rightConcat.value = { kind: 'field', field }
            syncRightConcat()
        }

        function updateRightConcatValue(value: string) {
            rightConcat.value = { kind: 'const_string', value }
            syncRightConcat()
        }

        function syncLeftConcat() {
            if (props.modelValue.type === 'concat') {
                const updated: Transform = {
                    ...props.modelValue,
                    left: { ...leftConcat.value },
                }
                emit('update:modelValue', updated)
            }
        }

        function syncRightConcat() {
            if (props.modelValue.type === 'concat') {
                const updated: Transform = {
                    ...props.modelValue,
                    right: { ...rightConcat.value },
                }
                emit('update:modelValue', updated)
            }
        }

        return {
            t,
            stringOperandKinds,
            concatSeparator,
            leftConcat,
            rightConcat,
            updateField,
            updateLeftConcatKind,
            updateLeftConcatField,
            updateLeftConcatValue,
            updateRightConcatKind,
            updateRightConcatField,
            updateRightConcatValue,
        }
    },
})
