<template>
    <v-container fluid>
        <v-row>
            <v-col cols="12">
                <v-card>
                    <v-card-title
                        v-if="title || $slots.actions"
                        class="d-flex align-center justify-space-between pa-4"
                    >
                        <div
                            v-if="title"
                            class="d-flex align-center"
                        >
                            <slot name="icon">
                                <SmartIcon
                                    v-if="icon"
                                    :icon="icon"
                                    size="lg"
                                    class="mr-3"
                                />
                            </slot>
                            <span class="text-h4">{{ title }}</span>
                        </div>
                        <div
                            v-else
                            class="flex-grow-1"
                        />
                        <div class="d-flex gap-2">
                            <slot name="actions" />
                        </div>
                    </v-card-title>

                    <v-card-text>
                        <slot />
                    </v-card-text>
                </v-card>
            </v-col>
        </v-row>
    </v-container>
</template>

<script setup lang="ts">
    import SmartIcon from '@/components/common/SmartIcon.vue'

    interface Props {
        title?: string
        icon?: string
    }

    defineProps<Props>()
</script>

<style scoped>
    /* Ensure container doesn't cause scrollbar layout shifts */
    :deep(.v-container) {
        scrollbar-gutter: stable;
    }
</style>
