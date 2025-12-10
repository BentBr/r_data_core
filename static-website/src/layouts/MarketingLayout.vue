<template>
    <div class="layout">
        <Header />
        <main>
            <slot />
        </main>
        <Footer />
    </div>
</template>

<script setup lang="ts">
    import Header from '@/components/Header.vue'
    import Footer from '@/components/Footer.vue'
</script>

<style scoped>
    .layout {
        min-height: 100vh;
        display: flex;
        flex-direction: column;
        background: rgb(var(--v-theme-background));
        color: rgb(var(--v-theme-on-background));
    }

    main {
        flex: 1;
    }
</style>
