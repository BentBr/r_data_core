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
                    v-for="(pair, idx) in localPairs"
                    :key="idx"
                >
                    <td>
                        <v-select
                            v-if="useSelectForLeft"
                            :model-value="pair.k"
                            :items="leftItems || []"
                            density="comfortable"
                            variant="underlined"
                            @update:model-value="(v: string) => updatePair(idx, { ...pair, k: v })"
                        />
                        <v-text-field
                            v-else
                            :model-value="pair.k"
                            density="comfortable"
                            variant="underlined"
                            @update:model-value="(v: string) => updatePair(idx, { ...pair, k: v })"
                        />
                    </td>
                    <td>
                        <v-select
                            v-if="useSelectForRight"
                            :model-value="pair.v"
                            :items="rightItems || []"
                            density="comfortable"
                            variant="underlined"
                            @update:model-value="(v: string) => updatePair(idx, { ...pair, v })"
                        />
                        <v-text-field
                            v-else
                            :model-value="pair.v"
                            density="comfortable"
                            variant="underlined"
                            @update:model-value="(v: string) => updatePair(idx, { ...pair, v })"
                        />
                    </td>
                    <td class="text-right">
                        <v-btn
                            size="x-small"
                            variant="text"
                            color="error"
                            @click="deletePair(idx)"
                        >
                            <SmartIcon
                                icon="trash-2"
                                :size="16"
                            />
                        </v-btn>
                    </td>
                </tr>
            </tbody>
        </v-table>
    </div>
</template>

<script setup lang="ts">
    import SmartIcon from '@/components/common/SmartIcon.vue'
    import { ref, watch, nextTick } from 'vue'
    import type { Mapping } from './dsl-utils'
    import { getMappingPairs, pairsToMapping } from './dsl-utils'

    type Pair = { k: string; v: string }

    const props = defineProps<{
        modelValue: Mapping
        leftLabel: string
        rightLabel: string
        leftItems?: string[]
        useSelectForLeft?: boolean
        rightItems?: string[]
        useSelectForRight?: boolean
    }>()

    const emit = defineEmits<{
        (e: 'update:modelValue', value: Mapping): void
        (e: 'add-pair'): void
    }>()

    // Expose method to add a new empty pair
    function addEmptyPair() {
        localPairs.value.push({ k: '', v: '' })
        // Don't emit immediately - let user type first
        // The update will happen when they type
    }

    // Expose addEmptyPair for parent components
    defineExpose({ addEmptyPair })

    const localPairs = ref<Pair[]>([])
    let isUpdatingFromLocal = false

    // Update local pairs when prop changes (but not when we're updating from local changes)
    watch(
        () => props.modelValue,
        newMapping => {
            if (isUpdatingFromLocal) {
                return
            }
            const newPairs = getMappingPairs(newMapping || {})
            // Preserve any empty pairs that are currently being edited
            const currentEmptyPairs = localPairs.value.filter(p => !p.k && !p.v)
            // Only update if actually different to prevent loops
            const currentPairsStr = JSON.stringify(localPairs.value.filter(p => p.k || p.v))
            const newPairsStr = JSON.stringify(newPairs.filter(p => p.k || p.v))
            if (currentPairsStr !== newPairsStr) {
                // Combine new pairs with any empty pairs being edited
                localPairs.value = [...newPairs, ...currentEmptyPairs].map(p => ({ ...p }))
            }
        },
        { immediate: true }
    )

    // Update parent when local pairs change
    function updatePair(idx: number, pair: Pair) {
        localPairs.value[idx] = { ...pair }
        // Emit update without triggering watch
        isUpdatingFromLocal = true
        const newMapping = pairsToMapping(localPairs.value)
        emit('update:modelValue', newMapping)
        // Reset flag after next tick to allow external updates
        void nextTick(() => {
            isUpdatingFromLocal = false
        })
    }

    function deletePair(idx: number) {
        localPairs.value.splice(idx, 1)
        void nextTick(() => {
            const newMapping = pairsToMapping(localPairs.value)
            emit('update:modelValue', newMapping)
        })
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
