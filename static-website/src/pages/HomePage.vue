<template>
    <div>
        <HeroSection
            :eyebrow="t('hero.eyebrow')"
            :title="t('hero.title')"
            :subtitle="t('hero.subtitle')"
            :primary-label="t('hero.primary')"
            :secondary-label="t('hero.secondary')"
            @primary="openDemo"
            @secondary="openContact"
        >
            <template #illustration>
                <v-card
                    class="hero-card"
                    elevation="6"
                >
                    <div class="hero-grid">
                        <div class="hero-row">
                            <SmartIcon icon="database" />
                            <div>
                                <p class="label">Entity Types</p>
                                <p class="value">32</p>
                            </div>
                        </div>
                        <div class="hero-row">
                            <SmartIcon icon="workflow" />
                            <div>
                                <p class="label">Workflows</p>
                                <p class="value">12</p>
                            </div>
                        </div>
                        <div class="hero-row">
                            <SmartIcon icon="shield" />
                            <div>
                                <p class="label">Policies</p>
                                <p class="value">RBAC</p>
                            </div>
                        </div>
                    </div>
                </v-card>
            </template>
        </HeroSection>

        <Section id="features">
            <header class="section-heading">
                <p class="eyebrow">{{ t('features.title') }}</p>
                <h2>{{ t('features.title') }}</h2>
            </header>
            <v-row dense>
                <v-col
                    v-for="feature in featureItems"
                    :key="feature.title"
                    cols="12"
                    md="4"
                >
                    <FeatureCard
                        :icon="feature.icon"
                        :title="feature.title"
                        :description="feature.description"
                    />
                </v-col>
            </v-row>
        </Section>

        <Section id="benefits">
            <header class="section-heading">
                <p class="eyebrow">{{ t('benefits.title') }}</p>
                <h2>{{ t('benefits.title') }}</h2>
            </header>
            <ul class="benefits">
                <li
                    v-for="benefit in benefitItems"
                    :key="benefit"
                >
                    <SmartIcon
                        icon="check-circle"
                        size="18"
                    />
                    <span>{{ benefit }}</span>
                </li>
            </ul>
        </Section>

        <Section id="pricing">
            <header class="section-heading">
                <p class="eyebrow">{{ t('pricing.title') }}</p>
                <h2>{{ t('pricing.title') }}</h2>
                <p class="subtitle">{{ t('pricing.subtitle') }}</p>
            </header>
            <ul class="benefits">
                <li
                    v-for="bullet in pricingBullets"
                    :key="bullet"
                >
                    <SmartIcon
                        icon="arrow-right"
                        size="18"
                    />
                    <span>{{ bullet }}</span>
                </li>
            </ul>
        </Section>

        <CTASection
            :eyebrow="t('cta.eyebrow')"
            :title="t('cta.title')"
            :subtitle="t('cta.subtitle')"
            :primary-label="t('cta.primary')"
            :secondary-label="t('cta.secondary')"
            @primary="openDemo"
            @secondary="openContact"
        >
            <v-sheet
                class="cta-stats"
                rounded="lg"
                elevation="2"
            >
                <div>
                    <p class="label">APIs</p>
                    <p class="value">REST + GraphQL</p>
                </div>
                <div>
                    <p class="label">Deploy</p>
                    <p class="value">Cloud / On-Prem</p>
                </div>
            </v-sheet>
        </CTASection>

        <DemoCredentialsDialog
            :model-value="showDemoDialog"
            :title="t('cta.demo_overlay.title')"
            :hint="t('cta.demo_overlay.hint')"
            :username-label="t('cta.demo_overlay.username')"
            :password-label="t('cta.demo_overlay.password')"
            :cancel-label="t('cta.demo_overlay.cancel')"
            :open-demo-label="t('cta.demo_overlay.open')"
            @update:model-value="showDemoDialog = $event"
            @open-demo="trackDemoOpen"
        />
    </div>
</template>

<script setup lang="ts">
    import { ref, computed } from 'vue'
    import HeroSection from '@/components/HeroSection.vue'
    import Section from '@/components/Section.vue'
    import FeatureCard from '@/components/FeatureCard.vue'
    import CTASection from '@/components/CTASection.vue'
    import DemoCredentialsDialog from '@/components/DemoCredentialsDialog.vue'
    import SmartIcon from '@/components/common/SmartIcon.vue'
    import { useTranslations } from '@/composables/useTranslations'
    import { useSEO } from '@/composables/useSEO'

    const { t, get } = useTranslations()
    const showDemoDialog = ref(false)

    const featureItems = computed(() => get<Array<Record<string, string>>>('features.items') ?? [])

    const benefitItems = computed(() => get<string[]>('benefits.items') ?? [])
    const pricingBullets = computed(() => get<string[]>('pricing.bullets') ?? [])

    const openDemo = () => {
        showDemoDialog.value = true
    }

    const openContact = () => {
        window.location.href = 'mailto:hello@rdatacore.example'
    }

    const trackDemoOpen = () => {
        // placeholder for analytics hook
    }

    useSEO({
        title: 'RDataCore',
        description: t('hero.subtitle'),
        locale: 'en', // Will use currentLanguage.value from composable
    })
</script>

<style scoped>
    .section-heading {
        margin-bottom: 24px;
    }

    .section-heading h2 {
        margin: 6px 0;
        font-size: clamp(1.6rem, 1.5vw + 1rem, 2rem);
    }

    .subtitle {
        margin: 0;
        color: rgb(var(--v-theme-on-surface-variant));
    }

    .eyebrow {
        margin: 0;
        font-weight: 600;
        color: rgb(var(--v-theme-primary));
        letter-spacing: 0.08em;
        text-transform: uppercase;
    }

    .hero-card {
        padding: 20px;
        min-width: 280px;
    }

    .hero-grid {
        display: grid;
        gap: 12px;
    }

    .hero-row {
        display: flex;
        align-items: center;
        gap: 12px;
        padding: 12px;
        border-radius: 12px;
        background: rgba(var(--v-theme-primary), 0.06);
    }

    .label {
        margin: 0;
        color: rgb(var(--v-theme-on-surface-variant));
        font-size: 0.85rem;
    }

    .value {
        margin: 2px 0 0;
        font-weight: 700;
    }

    .benefits {
        list-style: none;
        padding: 0;
        margin: 0;
        display: grid;
        gap: 12px;
    }

    .benefits li {
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 12px 14px;
        border-radius: 12px;
        background: rgba(var(--v-theme-primary), 0.04);
    }

    .cta-stats {
        display: grid;
        gap: 8px;
        padding: 16px;
        min-width: 220px;
    }
</style>
