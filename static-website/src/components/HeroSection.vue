<template>
    <Section>
        <div class="hero">
            <div class="hero__content">
                <p class="eyebrow">{{ eyebrow }}</p>
                <h1>{{ title }}</h1>
                <p class="subtitle">{{ subtitle }}</p>
                <div class="actions">
                    <v-btn
                        color="primary"
                        size="large"
                        @click="$emit('primary')"
                    >
                        {{ primaryLabel }}
                    </v-btn>
                    <v-btn
                        variant="text"
                        size="large"
                        @click="$emit('secondary')"
                    >
                        {{ secondaryLabel }}
                    </v-btn>
                </div>
            </div>
            <div class="hero__illustration">
                <slot name="illustration" />
            </div>
        </div>
    </Section>
</template>

<script setup lang="ts">
    import Section from './Section.vue'

    defineProps<{
        eyebrow: string
        title: string
        subtitle: string
        primaryLabel: string
        secondaryLabel: string
    }>()

    defineEmits<{
        (e: 'primary'): void
        (e: 'secondary'): void
    }>()
</script>

<style scoped>
    .hero {
        display: grid;
        gap: 32px;
        align-items: center;
        grid-template-columns: 1fr;
    }

    @media (min-width: 960px) {
        .hero {
            grid-template-columns: 1.1fr 0.9fr;
            gap: 48px;
        }
    }

    .hero__content h1 {
        margin: 8px 0 12px;
        font-size: clamp(2rem, 3vw + 1rem, 3rem);
        line-height: 1.1;
    }

    .subtitle {
        margin: 0 0 24px;
        color: rgb(var(--v-theme-on-surface-variant));
        font-size: 1.1rem;
        line-height: 1.6;
    }

    .eyebrow {
        margin: 0;
        font-weight: 600;
        color: rgb(var(--v-theme-primary));
        letter-spacing: 0.08em;
        text-transform: uppercase;
    }

    .actions {
        display: flex;
        gap: 12px;
        flex-wrap: wrap;
    }

    .hero__illustration {
        display: flex;
        justify-content: center;
    }
</style>
