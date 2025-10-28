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
                    variant="text"
                    class="mr-1"
                    @click="action.onClick?.(item)"
                />
            </div>
        </template>

        <template #item.status="{ item }">
            <v-chip
                :color="getStatusColor(item.status)"
                size="small"
            >
                {{ item.status }}
            </v-chip>
        </template>

        <template #item.published="{ item }">
            <v-icon
                :icon="item.published ? 'mdi-check-circle' : 'mdi-close-circle'"
                :color="item.published ? 'success' : 'grey'"
                size="small"
            />
        </template>

        <template #item.required="{ item }">
            <v-icon
                :icon="item.required ? 'mdi-check-circle' : 'mdi-close-circle'"
                :color="item.required ? 'success' : 'grey'"
                size="small"
            />
        </template>

        <template #item.indexed="{ item }">
            <v-icon
                :icon="item.indexed ? 'mdi-check-circle' : 'mdi-close-circle'"
                :color="item.indexed ? 'success' : 'grey'"
                size="small"
            />
        </template>

        <template #item.filterable="{ item }">
            <v-icon
                :icon="item.filterable ? 'mdi-check-circle' : 'mdi-close-circle'"
                :color="item.filterable ? 'success' : 'grey'"
                size="small"
            />
        </template>

        <template #item.field_type="{ item }">
            <v-chip
                size="x-small"
                color="primary"
            >
                {{ item.field_type }}
            </v-chip>
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
    interface Props {
        headers: Array<{ title: string; key: string; sortable?: boolean; align?: string }>
        items: Array<Record<string, unknown>>
        loading?: boolean
        itemsPerPage?: number
        page?: number
        totalItems?: number
        actions?: Array<{
            icon: string
            color?: string
            disabled?: boolean
            loading?: boolean
            onClick?: (item: Record<string, unknown>) => void
        }>
    }

    interface Emits {
        (e: 'update:options', options: Record<string, unknown>): void
    }

    defineProps<Props>()
    const emit = defineEmits<Emits>()

    const handleOptionsUpdate = (options: Record<string, unknown>) => {
        emit('update:options', options)
    }

    const getStatusColor = (status: string) => {
        const colorMap: Record<string, string> = {
            active: 'success',
            inactive: 'error',
            expired: 'warning',
            revoked: 'error',
        }
        return colorMap[status] || 'grey'
    }

    const formatDate = (dateString?: string) => {
        if (!dateString) {
            return 'N/A'
        }
        return new Date(dateString).toLocaleDateString()
    }
</script>
