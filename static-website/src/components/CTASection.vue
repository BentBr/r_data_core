<template>
    <Section>
        <v-card
            class="cta"
            elevation="3"
        >
            <div class="cta__content">
                <p class="eyebrow">{{ eyebrow }}</p>
                <h2>{{ title }}</h2>
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
                        variant="outlined"
                        size="large"
                        @click="$emit('secondary')"
                    >
                        {{ secondaryLabel }}
                    </v-btn>
                </div>
            </div>
            <div class="cta__side">
                <slot />
            </div>
        </v-card>
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
    .cta {
        display: grid;
        gap: 32px;
        grid-template-columns: 1fr;
        padding: 32px;
        border-radius: 16px;
    }

    @media (min-width: 960px) {
        .cta {
            grid-template-columns: 1.1fr 0.9fr;
            align-items: center;
        }
    }

    .cta__content h2 {
        margin: 8px 0 12px;
        font-size: clamp(1.5rem, 2vw + 1rem, 2.4rem);
    }

    .subtitle {
        margin: 0 0 20px;
        color: rgb(var(--v-theme-on-surface-variant));
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

    .cta__side {
        display: flex;
        justify-content: flex-end;
    }
</style>
