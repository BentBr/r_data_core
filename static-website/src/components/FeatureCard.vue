<template>
    <v-card
        class="feature-card"
        elevation="2"
    >
        <div class="icon">
            <SmartIcon
                :icon="icon"
                size="24"
            />
        </div>
        <h3>{{ title }}</h3>
        <p>{{ description }}</p>
    </v-card>
</template>

<script setup lang="ts">
    import SmartIcon from './common/SmartIcon.vue'

    defineProps<{
        icon: string
        title: string
        description: string
    }>()
</script>

<style scoped>
    .feature-card {
        padding: 20px;
        height: 100%;
        display: flex;
        flex-direction: column;
        gap: 12px;
    }

    .icon {
        width: 40px;
        height: 40px;
        border-radius: 12px;
        background: rgba(var(--v-theme-primary), 0.12);
        color: rgb(var(--v-theme-primary));
        display: grid;
        place-items: center;
    }

    h3 {
        margin: 0;
        font-size: 1.1rem;
    }

    p {
        margin: 0;
        color: rgb(var(--v-theme-on-surface-variant));
        line-height: 1.5;
    }
</style>
