<template>
    <div>
        <!-- Skeleton loader for initial load -->
        <div v-if="loading && items.length === 0">
            <v-skeleton-loader
                type="table"
                class="elevation-1"
            />
        </div>

        <!-- Error State -->
        <v-alert
            v-else-if="error"
            type="error"
            variant="tonal"
            class="mb-4"
            closable
        >
            {{ error }}
        </v-alert>

        <!-- Data Table with skeleton rows during loading -->
        <v-data-table-server
            v-else
            v-model:options="tableOptions"
            :headers="headers"
            :items="items"
            :loading="loading"
            :items-per-page="itemsPerPage"
            :page="currentPage"
            :items-length="totalItems"
            class="elevation-1"
            responsive
            :items-per-page-options="itemsPerPageOptions"
            :items-per-page-text="t('table.items_per_page')"
            :next-page-text="t('table.next')"
            :prev-page-text="t('table.previous')"
            :first-page-text="t('table.first')"
            :last-page-text="t('table.last')"
            :page-text="t('table.page')"
            :of-text="t('table.of')"
            :no-data-text="t('table.no_data')"
            :loading-text="t('table.loading')"
            item-value="uuid"
            @update:options="handleOptionsUpdate"
        >
            <!-- Slot for custom item templates -->
            <template
                v-for="(_, name) in $slots"
                #[name]="slotData"
            >
                <slot
                    :name="name"
                    v-bind="slotData"
                />
            </template>
        </v-data-table-server>
    </div>
</template>

<script setup lang="ts">
    import { useTranslations } from '@/composables/useTranslations'
    import { ref, watch } from 'vue'

    const { t } = useTranslations()

    interface Props {
        // Data
        items: unknown[]
        headers: unknown[]

        // Loading and error states
        loading: boolean
        error?: string
        loadingText?: string

        // Pagination
        currentPage: number
        itemsPerPage: number
        totalItems: number
        totalPages: number
        hasNext?: boolean
        hasPrevious?: boolean

        // Options
        itemsPerPageOptions?: number[]
    }

    interface Emits {
        (e: 'update:page', page: number): void
        (e: 'update:items-per-page', itemsPerPage: number): void
    }

    const props = withDefaults(defineProps<Props>(), {
        loadingText: 'Loading...',
        itemsPerPageOptions: () => [10, 25, 50, 100, 500],
    })

    const emit = defineEmits<Emits>()

    // Reactive table options
    const tableOptions = ref({
        page: props.currentPage,
        itemsPerPage: props.itemsPerPage,
    })

    // Watch for prop changes and update tableOptions
    watch(
        () => [props.currentPage, props.itemsPerPage],
        ([page, itemsPerPage]) => {
            tableOptions.value = {
                page: page as number,
                itemsPerPage: itemsPerPage as number,
            }
        },
        { immediate: true }
    )

    // Methods
    const handleOptionsUpdate = (options: { page: number; itemsPerPage: number }) => {
        if (options.page !== props.currentPage) {
            emit('update:page', options.page)
        }
        if (options.itemsPerPage !== props.itemsPerPage) {
            emit('update:items-per-page', options.itemsPerPage)
        }
    }
</script>
