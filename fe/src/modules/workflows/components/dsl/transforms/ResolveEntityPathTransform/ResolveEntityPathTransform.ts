import { computed, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import Badge from '@/shared/components/Badge/index.vue'
import type { Transform } from '../../contracts'

export default defineComponent({
    name: 'ResolveEntityPathTransform',
    components: {
        SmartIcon,
        Badge,
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

        const stringOperandKinds = ['field', 'const_string']

        const resolveEntityPathTarget = computed(() => {
            if (props.modelValue.type === 'resolve_entity_path') {
                return props.modelValue.target_path
            }
            return ''
        })

        const resolveEntityPathTargetUuid = computed(() => {
            if (props.modelValue.type === 'resolve_entity_path') {
                return props.modelValue.target_uuid ?? ''
            }
            return ''
        })

        const resolveEntityPathEntityType = computed(() => {
            if (props.modelValue.type === 'resolve_entity_path') {
                return props.modelValue.entity_type
            }
            return ''
        })

        const resolveEntityPathFilters = computed(() => {
            if (props.modelValue.type === 'resolve_entity_path') {
                return props.modelValue.filters
            }
            return {}
        })

        const resolveEntityPathFallback = computed(() => {
            if (props.modelValue.type === 'resolve_entity_path') {
                return props.modelValue.fallback_path ?? ''
            }
            return ''
        })

        function addFilter() {
            if (props.modelValue.type === 'resolve_entity_path') {
                const filters = { ...props.modelValue.filters }
                filters[''] = { kind: 'field', field: '' }
                const updated: Transform = {
                    ...props.modelValue,
                    filters,
                }
                emit('update:modelValue', updated)
            }
        }

        function removeFilter(field: string) {
            if (props.modelValue.type === 'resolve_entity_path') {
                const filters = { ...props.modelValue.filters }
                delete filters[field]
                const updated: Transform = {
                    ...props.modelValue,
                    filters,
                }
                emit('update:modelValue', updated)
            }
        }

        function updateFilterKind(field: string, kind: 'field' | 'const_string') {
            if (props.modelValue.type === 'resolve_entity_path') {
                const filters = { ...props.modelValue.filters }
                if (kind === 'field') {
                    filters[field] = { kind: 'field', field: '' }
                } else {
                    filters[field] = { kind: 'const_string', value: '' }
                }
                const updated: Transform = {
                    ...props.modelValue,
                    filters,
                }
                emit('update:modelValue', updated)
            }
        }

        function updateFilterField(field: string, value: string) {
            if (props.modelValue.type === 'resolve_entity_path') {
                const filters = { ...props.modelValue.filters }
                if (filters[field].kind === 'field') {
                    filters[field] = { kind: 'field', field: value }
                }
                const updated: Transform = {
                    ...props.modelValue,
                    filters,
                }
                emit('update:modelValue', updated)
            }
        }

        function updateFilterValue(field: string, value: string) {
            if (props.modelValue.type === 'resolve_entity_path') {
                const filters = { ...props.modelValue.filters }
                if (filters[field].kind === 'const_string') {
                    filters[field] = { kind: 'const_string', value }
                }
                const updated: Transform = {
                    ...props.modelValue,
                    filters,
                }
                emit('update:modelValue', updated)
            }
        }

        function updateResolveEntityPathField(field: string, value: unknown) {
            if (props.modelValue.type === 'resolve_entity_path') {
                const updated: Transform = {
                    ...props.modelValue,
                    [field]: value ?? undefined,
                }
                emit('update:modelValue', updated)
            }
        }

        return {
            t,
            stringOperandKinds,
            resolveEntityPathTarget,
            resolveEntityPathTargetUuid,
            resolveEntityPathEntityType,
            resolveEntityPathFilters,
            resolveEntityPathFallback,
            addFilter,
            removeFilter,
            updateFilterKind,
            updateFilterField,
            updateFilterValue,
            updateResolveEntityPathField,
        }
    },
})
