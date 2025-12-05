<template>
    <v-container fluid>
        <v-row>
            <v-col cols="12">
                <v-card>
                    <v-card-title class="text-h4 pa-4">
                        <v-icon
                            icon="mdi-view-dashboard"
                            class="mr-3"
                        />
                        R Data Core Admin Dashboard
                    </v-card-title>
                    <v-card-text>
                        <v-row>
                            <v-col
                                cols="12"
                                md="4"
                            >
                                <v-card
                                    color="primary"
                                    variant="tonal"
                                >
                                    <v-card-text>
                                        <div class="text-h6">Entity Definitions</div>
                                        <div class="text-h3">{{ metrics.entityDefinitions }}</div>
                                    </v-card-text>
                                </v-card>
                            </v-col>
                            <v-col
                                cols="12"
                                md="4"
                            >
                                <v-card
                                    color="success"
                                    variant="tonal"
                                >
                                    <v-card-text>
                                        <div class="text-h6">Entities</div>
                                        <div class="text-h3">{{ metrics.entities }}</div>
                                    </v-card-text>
                                </v-card>
                            </v-col>
                            <v-col
                                cols="12"
                                md="4"
                            >
                                <v-card
                                    color="info"
                                    variant="tonal"
                                >
                                    <v-card-text>
                                        <div class="text-h6">API Keys</div>
                                        <div class="text-h3">{{ metrics.apiKeys }}</div>
                                    </v-card-text>
                                </v-card>
                            </v-col>
                        </v-row>
                    </v-card-text>
                </v-card>
            </v-col>
        </v-row>

        <!-- Quick Actions -->
        <v-row class="mt-4">
            <v-col cols="12">
                <v-card>
                    <v-card-title>Quick Actions</v-card-title>
                    <v-card-text>
                        <v-row>
                            <v-col
                                v-if="canAccessEntityDefinitions"
                                cols="12"
                                sm="6"
                                md="3"
                            >
                                <v-btn
                                    color="primary"
                                    variant="outlined"
                                    block
                                    prepend-icon="mdi-plus"
                                    @click="$router.push('/entity-definitions')"
                                >
                                    New Entity Definition
                                </v-btn>
                            </v-col>
                            <v-col
                                v-if="canAccessEntities"
                                cols="12"
                                sm="6"
                                md="3"
                            >
                                <v-btn
                                    color="success"
                                    variant="outlined"
                                    block
                                    prepend-icon="mdi-database-plus"
                                    @click="$router.push('/entities')"
                                >
                                    Create Entity
                                </v-btn>
                            </v-col>
                            <v-col
                                v-if="canAccessApiKeys"
                                cols="12"
                                sm="6"
                                md="3"
                            >
                                <v-btn
                                    color="info"
                                    variant="outlined"
                                    block
                                    prepend-icon="mdi-key-plus"
                                    @click="$router.push('/api-keys')"
                                >
                                    Generate API Key
                                </v-btn>
                            </v-col>
                            <v-col
                                v-if="canAccessSystem"
                                cols="12"
                                sm="6"
                                md="3"
                            >
                                <v-btn
                                    color="warning"
                                    variant="outlined"
                                    block
                                    prepend-icon="mdi-cog"
                                    @click="$router.push('/system')"
                                >
                                    System Settings
                                </v-btn>
                            </v-col>
                        </v-row>
                    </v-card-text>
                </v-card>
            </v-col>
        </v-row>
    </v-container>
</template>

<script setup lang="ts">
    import { ref, computed, onMounted } from 'vue'
    import { useAuthStore } from '@/stores/auth'

    const authStore = useAuthStore()

    // Permission checks for quick action buttons
    const canAccessEntityDefinitions = computed(() =>
        authStore.canAccessRoute('/entity-definitions')
    )
    const canAccessEntities = computed(() => authStore.canAccessRoute('/entities'))
    const canAccessApiKeys = computed(() => authStore.canAccessRoute('/api-keys'))
    const canAccessSystem = computed(() => authStore.canAccessRoute('/system'))

    // Mock data for now
    const metrics = ref({
        entityDefinitions: 0,
        entities: 0,
        apiKeys: 0,
    })

    // TODO: Fetch real metrics from API
    onMounted(async () => {
        // Mock data
        metrics.value = {
            entityDefinitions: 12,
            entities: 1542,
            apiKeys: 8,
        }
    })
</script>

<style scoped>
    /* Component-specific styles */
</style>
