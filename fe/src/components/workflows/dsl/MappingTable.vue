<template>
    <div class="mapping-table-wrapper">
        <v-table
            density="comfortable"
            class="mapping-table"
        >
            <thead>
                <tr>
                    <th style="width: 45%">{{ leftLabel }}</th>
                    <th style="width: 45%">{{ rightLabel }}</th>
                    <th style="width: 10%"></th>
                </tr>
            </thead>
            <tbody>
                <tr
                    v-for="(pair, idx) in pairs"
                    :key="idx"
                >
                    <td>
                        <v-text-field
                            v-model="localPairs[idx].k"
                            density="comfortable"
                            variant="outlined"
                            @update:model-value="emitUpdate(idx)"
                        />
                    </td>
                    <td>
                        <v-select
                            v-if="useSelectForRight"
                            v-model="localPairs[idx].v"
                            :items="rightItems || []"
                            density="comfortable"
                            variant="outlined"
                            @update:model-value="emitUpdate(idx)"
                        />
                        <v-text-field
                            v-else
                            v-model="localPairs[idx].v"
                            density="comfortable"
                            variant="outlined"
                            @update:model-value="emitUpdate(idx)"
                        />
                    </td>
                    <td class="text-right">
                        <v-btn
                            size="x-small"
                            :variant="buttonConfigs.text.variant"
                            :color="buttonConfigs.destructive.color"
                            @click="$emit('delete-pair', idx)"
                        >
                            <SmartIcon
                                icon="trash-2"
                                size="xs"
                            />
                        </v-btn>
                    </td>
                </tr>
            </tbody>
        </v-table>
    </div>
</template>

<script setup lang="ts">
    import { ref, watch } from 'vue'
    import { buttonConfigs } from '@/design-system/components'

    type Pair = { k: string; v: string }
    const props = defineProps<{
        pairs: Pair[]
        leftLabel: string
        rightLabel: string
        rightItems?: string[]
        useSelectForRight?: boolean
    }>()
    const emit = defineEmits<{
        (e: 'update-pair', idx: number, pair: Pair): void
        (e: 'delete-pair', idx: number): void
    }>()

    const localPairs = ref<Pair[]>([])

    watch(
        () => props.pairs,
        v => {
            localPairs.value = Array.isArray(v) ? v.map(p => ({ ...p })) : []
        },
        { immediate: true, deep: true }
    )

    function emitUpdate(idx: number) {
        const pair = localPairs.value[idx]
        emit('update-pair', idx, { ...pair })
    }
</script>

<style scoped>
    .mapping-table-wrapper {
        overflow-x: auto;
    }
    .mapping-table {
        min-width: 560px;
    }
</style>
