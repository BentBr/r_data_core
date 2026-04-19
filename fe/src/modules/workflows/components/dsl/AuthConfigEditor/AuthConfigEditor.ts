import { computed, ref, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import type { AuthConfig } from '../contracts'

export default defineComponent({
    name: 'AuthConfigEditor',
    props: {
        modelValue: { type: Object as PropType<AuthConfig>, required: true },
    },
    emits: ['update:modelValue'],
    setup(props, { emit }) {
        const { t } = useTranslations()
        const authType = computed(() => props.modelValue.type)
        const authTypes = [
            { title: t('workflows.dsl.auth_none'), value: 'none' },
            { title: t('workflows.dsl.auth_api_key'), value: 'api_key' },
            { title: t('workflows.dsl.auth_basic'), value: 'basic_auth' },
            { title: t('workflows.dsl.auth_pre_shared_key'), value: 'pre_shared_key' },
            { title: t('workflows.dsl.auth_entity_jwt'), value: 'entity_jwt' },
        ]
        const keyLocations = [
            { title: t('workflows.dsl.key_location_header'), value: 'header' },
            { title: t('workflows.dsl.key_location_body'), value: 'body' },
        ]
        const newClaimKey = ref('')
        const newClaimValue = ref('')
        const requiredClaims = computed(() => {
            if (props.modelValue.type === 'entity_jwt') return (props.modelValue as any).required_claims ?? {}
            return {}
        })
        function addRequiredClaim() {
            if (!newClaimKey.value) return
            const claims = { ...requiredClaims.value, [newClaimKey.value]: newClaimValue.value }
            emit('update:modelValue', { type: 'entity_jwt', required_claims: claims } as AuthConfig)
            newClaimKey.value = ''; newClaimValue.value = ''
        }
        function removeRequiredClaim(key: string) {
            const claims = { ...requiredClaims.value }; delete claims[key]
            emit('update:modelValue', { type: 'entity_jwt', required_claims: Object.keys(claims).length > 0 ? claims : undefined } as AuthConfig)
        }
        function updateRequiredClaim(key: string, value: string) {
            const claims = { ...requiredClaims.value, [key]: value }
            emit('update:modelValue', { type: 'entity_jwt', required_claims: claims } as AuthConfig)
        }
        function onAuthTypeChange(newType: string) {
            let newAuth: AuthConfig
            if (newType === 'api_key') newAuth = { type: 'api_key', key: '', header_name: 'X-API-Key' }
            else if (newType === 'basic_auth') newAuth = { type: 'basic_auth', username: '', password: '' }
            else if (newType === 'pre_shared_key') newAuth = { type: 'pre_shared_key', key: '', location: 'header', field_name: '' }
            else if (newType === 'entity_jwt') newAuth = { type: 'entity_jwt' } as AuthConfig
            else newAuth = { type: 'none' }
            emit('update:modelValue', newAuth)
        }
        function updateField(field: string, value: any) { emit('update:modelValue', { ...props.modelValue, [field]: value } as AuthConfig) }
        return {
            t, authType, authTypes, keyLocations, newClaimKey, newClaimValue, requiredClaims,
            addRequiredClaim, removeRequiredClaim, updateRequiredClaim, onAuthTypeChange, updateField,
        }
    },
})
