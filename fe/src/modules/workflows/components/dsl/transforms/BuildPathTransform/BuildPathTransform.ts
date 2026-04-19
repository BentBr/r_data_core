import { computed, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import MappingTable from '../../MappingTable/index.vue'
import type { Transform } from '../../contracts'

export default defineComponent({
    name: 'BuildPathTransform',
    components: {
        MappingTable,
    },
    props: {
        modelValue: {
            type: Object as PropType<Transform>,
            required: true,
        },
    },
    emits: ['update:modelValue'],
    setup(props, { emit }) {
        const { t } = useTranslations()

        const buildPathTarget = computed(() => {
            if (props.modelValue.type === 'build_path') {
                return props.modelValue.target
            }
            return ''
        })

        const buildPathTemplate = computed(() => {
            if (props.modelValue.type === 'build_path') {
                return props.modelValue.template
            }
            return ''
        })

        const buildPathSeparator = computed(() => {
            if (props.modelValue.type === 'build_path') {
                return props.modelValue.separator ?? '/'
            }
            return '/'
        })

        const buildPathFieldTransforms = computed({
            get() {
                if (props.modelValue.type === 'build_path') {
                    const transforms = props.modelValue.field_transforms ?? {}
                    return Object.entries(transforms).map(([k, v]) => ({ k, v }))
                }
                return []
            },
            set(pairs: Array<{ k: string; v: string }>) {
                if (props.modelValue.type === 'build_path') {
                    const transforms: Record<string, string> = {}
                    for (const { k, v } of pairs) {
                        if (k && v) {
                            transforms[k] = v
                        }
                    }
                    const updated: Transform = {
                        ...props.modelValue,
                        field_transforms: Object.keys(transforms).length > 0 ? transforms : undefined,
                    }
                    emit('update:modelValue', updated)
                }
            },
        })

        function updateBuildPathField(field: string, value: unknown) {
            if (props.modelValue.type === 'build_path') {
                const updated: Transform = {
                    ...props.modelValue,
                    [field]: value,
                }
                emit('update:modelValue', updated)
            }
        }

        function updateBuildPathFieldTransform(idx: number, pair: { k: string; v: string }) {
            if (props.modelValue.type === 'build_path') {
                const currentPairs = buildPathFieldTransforms.value.map(p => ({ ...p }))
                currentPairs[idx] = pair
                const transforms: Record<string, string> = {}
                for (const { k, v } of currentPairs) {
                    if (k && v) {
                        transforms[k] = v
                    }
                }
                const updated: Transform = {
                    ...props.modelValue,
                    field_transforms: Object.keys(transforms).length > 0 ? transforms : undefined,
                }
                emit('update:modelValue', updated)
            }
        }

        function deleteBuildPathFieldTransform(idx: number) {
            if (props.modelValue.type === 'build_path') {
                const currentPairs = buildPathFieldTransforms.value.map(p => ({ ...p }))
                currentPairs.splice(idx, 1)
                const transforms: Record<string, string> = {}
                for (const { k, v } of currentPairs) {
                    if (k && v) {
                        transforms[k] = v
                    }
                }
                const updated: Transform = {
                    ...props.modelValue,
                    field_transforms: Object.keys(transforms).length > 0 ? transforms : undefined,
                }
                emit('update:modelValue', updated)
            }
        }

        function addBuildPathFieldTransform() {
            if (props.modelValue.type === 'build_path') {
                const currentPairs = buildPathFieldTransforms.value.map(p => ({ ...p }))
                currentPairs.push({ k: '', v: '' })
                const transforms: Record<string, string> = {}
                for (const { k, v } of currentPairs) {
                    if (k && v) {
                        transforms[k] = v
                    }
                }
                const updated: Transform = {
                    ...props.modelValue,
                    field_transforms: Object.keys(transforms).length > 0 ? transforms : undefined,
                }
                emit('update:modelValue', updated)
            }
        }

        return {
            t,
            buildPathTarget,
            buildPathTemplate,
            buildPathSeparator,
            buildPathFieldTransforms,
            updateBuildPathField,
            updateBuildPathFieldTransform,
            deleteBuildPathFieldTransform,
            addBuildPathFieldTransform,
        }
    },
})
