<template>
    <div class="login-layout">
        <slot />
    </div>
</template>

<script setup lang="ts">
    // Simple layout wrapper for login page that inherits theme
</script>

<style scoped>
    .login-layout {
        min-height: 100vh;
        width: 100%;
    }
</style>
