<template>
    <v-app>
        <MarketingLayout>
            <router-view />
        </MarketingLayout>
    </v-app>
</template>

<script setup lang="ts">
    import MarketingLayout from './layouts/MarketingLayout.vue'
</script>

<style>
    html,
    body {
        margin: 0;
        padding: 0;
        font-family: var(
            --v-theme-font-family,
            'Inter',
            system-ui,
            -apple-system,
            'Segoe UI',
            sans-serif
        );
    }
</style>
