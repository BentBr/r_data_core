import { ref, watch, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import EntityDefinitionMetaInfo from '../EntityDefinitionMetaInfo/index.vue'
import EntityDefinitionFields from '../EntityDefinitionFields/index.vue'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import VersionHistory, { type VersionHistoryInstance } from '@/shared/components/VersionHistory/VersionHistory'
import type { EntityDefinition } from '@/types/schemas'
import { typedHttpClient } from '@/api/typed-client'
import { computeDiffRows } from '@/utils/versionDiff'

export default defineComponent({
    name: 'EntityDefinitionDetails',
    components: {
        EntityDefinitionMetaInfo,
        EntityDefinitionFields,
        SmartIcon,
        VersionHistory,
    },
    props: {
        definition: { type: Object as PropType<EntityDefinition | null>, default: null },
        hasUnsavedChanges: { type: Boolean, required: true },
        savingChanges: { type: Boolean, required: true },
    },
    emits: ['edit', 'delete', 'save-changes', 'add-field', 'edit-field', 'remove-field'],
    setup(props, { emit }) {
        const { t } = useTranslations()
        const activeTab = ref('meta')
        const versions = ref<Array<{ version_number: number, created_at: string, created_by?: string | null, created_by_name?: string | null }>>([])
        const versionHistoryRef = ref<VersionHistoryInstance>()

        const loadVersions = async () => {
            if (!props.definition?.uuid) return
            try { versions.value = await typedHttpClient.listEntityDefinitionVersions(props.definition.uuid) }
            catch (e) { console.error('Failed to load versions:', e) }
        }

        const handleVersionCompare = async (versionA: number, versionB: number) => {
            if (!props.definition?.uuid) return
            try {
                const [a, b] = await Promise.all([
                    typedHttpClient.getEntityDefinitionVersion(props.definition.uuid, versionA),
                    typedHttpClient.getEntityDefinitionVersion(props.definition.uuid, versionB),
                ])
                versionHistoryRef.value?.updateDiffRows(computeDiffRows(a.data as any, b.data as any))
            } catch (e) { console.error('Failed to load diff:', e) }
        }

        watch(() => props.definition?.uuid, async () => { await loadVersions() }, { immediate: true })
        watch(() => activeTab.value, async newTab => { if (newTab === 'history' && props.definition?.uuid) await loadVersions() })

        return { t, activeTab, versions, versionHistoryRef, handleVersionCompare, emit }
    },
})
