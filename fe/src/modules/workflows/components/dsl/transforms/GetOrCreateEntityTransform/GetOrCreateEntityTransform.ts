import { computed, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import type { Transform } from '../../contracts'

export default defineComponent({
    name: 'GetOrCreateEntityTransform',
    props: {
        modelValue: {
            type: Object as PropType<Transform>,
            required: true,
        },
    },
    emits: ['update:modelValue'],
    setup(props, { emit }) {
        const { t } = useTranslations()

        const getOrCreateTargetPath = computed(() => {
            if (props.modelValue.type === 'get_or_create_entity') {
                return props.modelValue.target_path
            }
            return ''
        })

        const getOrCreateTargetUuid = computed(() => {
            if (props.modelValue.type === 'get_or_create_entity') {
                return props.modelValue.target_uuid ?? ''
            }
            return ''
        })

        const getOrCreateEntityType = computed(() => {
            if (props.modelValue.type === 'get_or_create_entity') {
                return props.modelValue.entity_type
            }
            return ''
        })

        const getOrCreatePathTemplate = computed(() => {
            if (props.modelValue.type === 'get_or_create_entity') {
                return props.modelValue.path_template
            }
            return ''
        })

        const getOrCreatePathSeparator = computed(() => {
            if (props.modelValue.type === 'get_or_create_entity') {
                return props.modelValue.path_separator ?? '/'
            }
            return '/'
        })

        function updateGetOrCreateField(field: string, value: unknown) {
            if (props.modelValue.type === 'get_or_create_entity') {
                const updated: Transform = {
                    ...props.modelValue,
                    [field]: value ?? undefined,
                }
                emit('update:modelValue', updated)
            }
        }

        return {
            t,
            getOrCreateTargetPath,
            getOrCreateTargetUuid,
            getOrCreateEntityType,
            getOrCreatePathTemplate,
            getOrCreatePathSeparator,
            updateGetOrCreateField,
        }
    },
})
