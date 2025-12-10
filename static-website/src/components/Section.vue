<template>
    <section class="section">
        <div class="section-inner">
            <slot />
        </div>
    </section>
</template>

<style scoped>
    .section {
        padding: 64px 16px;
    }

    .section-inner {
        max-width: 1200px;
        margin: 0 auto;
    }

    @media (min-width: 960px) {
        .section {
            padding: 96px 24px;
        }
    }
</style>
