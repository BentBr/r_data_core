<template>
    <v-data-table
        :headers="headers"
        :items="items"
        :loading="loading"
        :items-per-page="itemsPerPage"
        :page="page"
        :server-items-length="totalItems"
        @update:options="handleOptionsUpdate"
    >
        <template #item.actions="{ item }">
            <div class="d-flex align-center">
                <v-btn
                    v-for="action in actions"
                    :key="action.icon"
                    :icon="action.icon"
                    :color="action.color ?? 'primary'"
                    :disabled="action.disabled ?? false"
                    :loading="action.loading ?? false"
                    size="x-small"
                    :variant="buttonConfigs.text.variant"
                    class="mr-1"
                    @click="action.onClick?.(item)"
                />
            </div>
        </template>

        <template #item.status="{ item }">
            <Badge
                :status="item.status"
                size="small"
            >
                {{ item.status }}
            </Badge>
        </template>

        <template #item.published="{ item }">
            <SmartIcon
                :icon="item.published ? 'check-circle' : 'x-circle'"
                :color="item.published ? 'success' : 'mutedForeground'"
                size="sm"
            />
        </template>

        <template #item.required="{ item }">
            <SmartIcon
                :icon="item.required ? 'check-circle' : 'x-circle'"
                :color="item.required ? 'success' : 'mutedForeground'"
                size="sm"
            />
        </template>

        <template #item.indexed="{ item }">
            <SmartIcon
                :icon="item.indexed ? 'check-circle' : 'x-circle'"
                :color="item.indexed ? 'success' : 'mutedForeground'"
                size="sm"
            />
        </template>

        <template #item.filterable="{ item }">
            <SmartIcon
                :icon="item.filterable ? 'check-circle' : 'x-circle'"
                :color="item.filterable ? 'success' : 'mutedForeground'"
                size="sm"
            />
        </template>

        <template #item.field_type="{ item }">
            <Badge
                color="primary"
                size="small"
            >
                {{ item.field_type }}
            </Badge>
        </template>

        <template #item.created_at="{ item }">
            {{ formatDate(item.created_at) }}
        </template>

        <template #item.updated_at="{ item }">
            {{ formatDate(item.updated_at) }}
        </template>

        <template #item.expires_at="{ item }">
            {{ item.expires_at ? formatDate(item.expires_at) : 'Never' }}
        </template>

        <template #item.last_used_at="{ item }">
            {{ item.last_used_at ? formatDate(item.last_used_at) : 'Never' }}
        </template>
    </v-data-table>
</template>

<script setup lang="ts">
    import SmartIcon from '@/components/common/SmartIcon.vue'
    import Badge from '@/components/common/Badge.vue'
    import { buttonConfigs } from '@/design-system/components'
    import type { TableRow, TableHeader, TableAction } from '@/types/common'

    interface Props {
        headers: TableHeader[]
        items: TableRow[]
        loading?: boolean
        itemsPerPage?: number
        page?: number
        totalItems?: number
        actions?: TableAction[]
    }

    interface Emits {
        (
            e: 'update:options',
            options: {
                page?: number
                itemsPerPage?: number
                sortBy?: Array<{ key: string; order: 'asc' | 'desc' }>
            }
        ): void
    }

    defineProps<Props>()
    const emit = defineEmits<Emits>()

    const handleOptionsUpdate = (options: Record<string, unknown>) => {
        emit('update:options', options)
    }

    const formatDate = (dateString?: string) => {
        if (!dateString) {
            return 'N/A'
        }
        return new Date(dateString).toLocaleDateString()
    }
</script>
