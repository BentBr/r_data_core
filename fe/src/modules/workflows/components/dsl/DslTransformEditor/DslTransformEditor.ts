import { ref, watch, computed, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import ArithmeticTransform from '../transforms/ArithmeticTransform/index.vue'
import ConcatTransform from '../transforms/ConcatTransform/index.vue'
import BuildPathTransform from '../transforms/BuildPathTransform/index.vue'
import ResolveEntityPathTransform from '../transforms/ResolveEntityPathTransform/index.vue'
import GetOrCreateEntityTransform from '../transforms/GetOrCreateEntityTransform/index.vue'
import AuthenticateTransform from '../transforms/AuthenticateTransform/index.vue'
import type { Transform } from '../contracts'

export default defineComponent({
    name: 'DslTransformEditor',
    components: {
        ArithmeticTransform,
        ConcatTransform,
        BuildPathTransform,
        ResolveEntityPathTransform,
        GetOrCreateEntityTransform,
        AuthenticateTransform,
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

        const availableFields = computed(() => props.availableFields ?? [])

        const transformTypes = [
            { title: 'None', value: 'none' },
            { title: 'Arithmetic', value: 'arithmetic' },
            { title: 'Concat', value: 'concat' },
            { title: 'Build Path', value: 'build_path' },
            { title: 'Resolve Entity Path', value: 'resolve_entity_path' },
            { title: 'Get or Create Entity', value: 'get_or_create_entity' },
            { title: 'Authenticate', value: 'authenticate' },
        ]

        const transformType = ref<
            | 'none'
            | 'arithmetic'
            | 'concat'
            | 'build_path'
            | 'resolve_entity_path'
            | 'get_or_create_entity'
            | 'authenticate'
        >(props.modelValue.type)

        watch(
            () => props.modelValue.type,
            newType => {
                transformType.value = newType
            }
        )

        function onTypeChange(
            newType:
                | 'none'
                | 'arithmetic'
                | 'concat'
                | 'build_path'
                | 'resolve_entity_path'
                | 'get_or_create_entity'
                | 'authenticate'
        ) {
            transformType.value = newType
            let newTransform: Transform
            if (newType === 'none') {
                newTransform = { type: 'none' }
            } else if (newType === 'arithmetic') {
                newTransform = {
                    type: 'arithmetic',
                    target: '',
                    left: { kind: 'field', field: '' },
                    op: 'add',
                    right: { kind: 'const', value: 0 },
                }
            } else if (newType === 'concat') {
                newTransform = {
                    type: 'concat',
                    target: '',
                    left: { kind: 'field', field: '' },
                    separator: ' ',
                    right: { kind: 'field', field: '' },
                }
            } else if (newType === 'build_path') {
                newTransform = {
                    type: 'build_path',
                    target: '',
                    template: '',
                    separator: '/',
                }
            } else if (newType === 'resolve_entity_path') {
                newTransform = {
                    type: 'resolve_entity_path',
                    target_path: '',
                    entity_type: '',
                    filters: {},
                }
            } else if (newType === 'get_or_create_entity') {
                newTransform = {
                    type: 'get_or_create_entity',
                    target_path: '',
                    entity_type: '',
                    path_template: '',
                }
            } else {
                // newType is 'authenticate' at this point
                newTransform = {
                    type: 'authenticate',
                    entity_type: '',
                    identifier_field: '',
                    password_field: '',
                    input_identifier: '',
                    input_password: '',
                    target_token: '',
                }
            }
            emit('update:modelValue', newTransform)
        }

        return {
            t,
            availableFields,
            transformTypes,
            transformType,
            onTypeChange,
            emit, // needed for template usage of emit if not using setup context
        }
    },
})
