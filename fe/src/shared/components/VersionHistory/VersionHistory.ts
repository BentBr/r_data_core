import { ref, watch, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'

interface Version {
    version_number: number
    created_at: string
    created_by?: string | null
    created_by_name?: string | null
}

interface DiffRow {
    field: string
    a: string
    b: string
    changed: boolean
}

export default defineComponent({
    name: 'VersionHistory',
    props: {
        versions: { type: Array as PropType<Version[]>, required: true },
        loading: { type: Boolean, default: false },
    },
    emits: ['compare'],
    setup(props, { emit }) {
        const { t } = useTranslations()
        const selectedVersions = ref<number[]>([])
        const diffRows = ref<DiffRow[]>([])

        const isVersionSelected = (versionNumber: number): boolean => selectedVersions.value.includes(versionNumber)

        const toggleVersionSelection = async (versionNumber: number) => {
            const index = selectedVersions.value.indexOf(versionNumber)
            if (index > -1) {
                selectedVersions.value.splice(index, 1)
                diffRows.value = []
            } else if (selectedVersions.value.length < 2) {
                selectedVersions.value.push(versionNumber)
                if (selectedVersions.value.length === 2) {
                    emit('compare', selectedVersions.value[0], selectedVersions.value[1])
                }
            }
        }

        const updateDiffRows = (rows: DiffRow[]) => { diffRows.value = rows }

        watch(() => props.versions, () => {
            selectedVersions.value = []
            diffRows.value = []
        })

        return {
            t, selectedVersions, diffRows, isVersionSelected, toggleVersionSelection, updateDiffRows,
        }
    },
})

export type VersionHistoryInstance = InstanceType<typeof import('./VersionHistory').default>
