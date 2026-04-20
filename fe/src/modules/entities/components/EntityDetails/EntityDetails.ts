import { ref, watch, defineComponent, PropType } from 'vue'
import JsonEditorVue from 'json-editor-vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import VersionHistory, { type VersionHistoryInstance } from '@/shared/components/VersionHistory/VersionHistory'
import type { DynamicEntity, EntityDefinition } from '@/types/schemas'
import { typedHttpClient } from '@/api/typed-client'
import { computeDiffRows } from '@/utils/versionDiff'

export default defineComponent({
    name: 'EntityDetails',
    components: {
        SmartIcon,
        VersionHistory,
        JsonEditorVue,
    },
    props: {
        entity: {
            type: Object as PropType<DynamicEntity | null>,
            default: null,
        },
        entityDefinition: {
            type: Object as PropType<EntityDefinition | null>,
            default: null,
        },
    },
    emits: ['edit', 'delete'],
    setup(props, { emit }) {
        const { t } = useTranslations()

        const toToken = (s: string): string =>
            (s || '')
                .toLowerCase()
                .split(/[^a-z0-9]+/g)
                .filter(Boolean)
                .join('')

        const resolveFieldValue = (data: Record<string, unknown>, fieldName: string): unknown => {
            if (fieldName in data) return data[fieldName]
            const lower = fieldName.toLowerCase()
            for (const k of Object.keys(data)) {
                if (k.toLowerCase() === lower) return data[k]
            }
            const wanted = toToken(fieldName)
            for (const k of Object.keys(data)) {
                if (toToken(k) === wanted) return data[k]
            }
            return undefined
        }

        const versions = ref<
            Array<{
                version_number: number
                created_at: string
                created_by?: string | null
                created_by_name?: string | null
            }>
        >([])
        const versionHistoryRef = ref<VersionHistoryInstance>()

        const loadVersions = async () => {
            if (!props.entity) return
            try {
                const uuid = String(props.entity.field_data.uuid ?? '')
                const entityType = props.entity.entity_type
                versions.value = await typedHttpClient.listEntityVersions(entityType, uuid)
            } catch (e) {
                console.error('Failed to load versions:', e)
            }
        }

        const handleVersionCompare = async (versionA: number, versionB: number) => {
            if (!props.entity) return
            const uuid = String(props.entity.field_data.uuid ?? '')
            const entityType = props.entity.entity_type
            try {
                const [a, b] = await Promise.all([
                    typedHttpClient.getEntityVersion(entityType, uuid, versionA),
                    typedHttpClient.getEntityVersion(entityType, uuid, versionB),
                ])
                const aData = a.data as Record<string, unknown>
                const bData = b.data as Record<string, unknown>
                const diffRows = computeDiffRows(aData, bData)
                versionHistoryRef.value?.updateDiffRows(diffRows)
            } catch (e) {
                console.error('Failed to load diff:', e)
            }
        }

        watch(() => props.entity?.field_data.uuid, async () => { await loadVersions() }, { immediate: true })

        const getFieldIcon = (fieldType: string) => {
            const iconMap: Record<string, string> = {
                String: 'type', Text: 'file-text', Wysiwyg: 'file-edit',
                Integer: 'hash', Float: 'hash', Boolean: 'check-square',
                Date: 'calendar', DateTime: 'calendar-clock', Time: 'clock',
                Email: 'mail', Url: 'link', File: 'file', Image: 'image',
                Json: 'braces', Object: 'box', Array: 'list', Uuid: 'hash',
                ManyToOne: 'link', ManyToMany: 'link-2',
                Select: 'list-checks', MultiSelect: 'list-checks', Password: 'lock',
            }
            return iconMap[fieldType] || 'type'
        }

        const formatFieldValue = (value: unknown, fieldType: string): string => {
            if (fieldType === 'Password') return '******'
            if (value === null || value === undefined) return t('common.empty')
            switch (fieldType) {
                case 'Boolean': return value === true ? t('common.yes') : t('common.no')
                case 'Date':
                case 'DateTime': return new Date(value as string).toLocaleDateString()
                case 'Time': return new Date(`2000-01-01T${value}`).toLocaleTimeString()
                case 'Json':
                case 'Object':
                case 'Array': return typeof value === 'object' ? JSON.stringify(value) : String(value)
                default: return typeof value === 'object' ? JSON.stringify(value) : String(value)
            }
        }

        const formatDate = (value: string | undefined | null): string => {
            if (!value) return ''
            const date = new Date(value)
            return Number.isNaN(date.getTime()) ? '' : date.toLocaleString()
        }

        return {
            t,
            versions,
            versionHistoryRef,
            resolveFieldValue,
            handleVersionCompare,
            getFieldIcon,
            formatFieldValue,
            formatDate,
            emit,
        }
    },
})
