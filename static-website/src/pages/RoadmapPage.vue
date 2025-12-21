<template>
    <div class="roadmap-page">
        <Section class="hero-section">
            <div class="roadmap-container">
                <div class="hero-content">
                    <div class="badge">{{ t('roadmap.hero.badge') }}</div>
                    <h1>{{ t('roadmap.hero.title') }}</h1>
                    <p class="subtitle">{{ t('roadmap.hero.subtitle') }}</p>
                </div>
            </div>
        </Section>

        <Section class="features-section">
            <div class="roadmap-container">
                <div class="section-header">
                    <h2>{{ t('roadmap.done.title') }}</h2>
                    <p class="section-subtitle">{{ t('roadmap.done.subtitle') }}</p>
                </div>
                <div class="features-grid">
                    <div
                        v-for="(_feature, index) in doneFeatures"
                        :key="`done-${index}`"
                        class="feature-card done"
                    >
                        <div class="feature-icon">✓</div>
                        <h3>{{ t(`roadmap.done.features.${index}.title`) }}</h3>
                        <p>{{ t(`roadmap.done.features.${index}.desc`) }}</p>
                    </div>
                </div>
            </div>
        </Section>

        <Section class="features-section">
            <div class="roadmap-container">
                <div class="section-header">
                    <h2>{{ t('roadmap.open.title') }}</h2>
                    <p class="section-subtitle">{{ t('roadmap.open.subtitle') }}</p>
                </div>
                <div class="features-grid">
                    <div
                        v-for="(_feature, index) in openFeatures"
                        :key="`open-${index}`"
                        class="feature-card open"
                    >
                        <div class="feature-icon">→</div>
                        <h3>{{ t(`roadmap.open.features.${index}.title`) }}</h3>
                        <p>{{ t(`roadmap.open.features.${index}.desc`) }}</p>
                    </div>
                </div>
            </div>
        </Section>

        <Section class="wish-section">
            <div class="roadmap-container">
                <div class="section-header">
                    <h2>{{ t('roadmap.wish.title') }}</h2>
                    <p class="section-subtitle">{{ t('roadmap.wish.subtitle') }}</p>
                </div>
                <div class="wish-grid">
                    <div
                        v-for="(item, index) in wishItems"
                        :key="`wish-${index}`"
                        class="wish-card"
                        @click="openContact"
                    >
                        <div class="wish-icon">{{ item.icon }}</div>
                        <h3>{{ t(`roadmap.wish.items.${index}.title`) }}</h3>
                        <p>{{ t(`roadmap.wish.items.${index}.desc`) }}</p>
                        <div class="wish-cta">
                            {{ t('roadmap.wish.cta') }}
                        </div>
                    </div>
                </div>
            </div>
        </Section>

        <DemoCredentialsDialog
            :model-value="showDemoDialog"
            :title="t('cta.demo_overlay.title')"
            :hint="t('cta.demo_overlay.hint')"
            :username-label="t('cta.demo_overlay.username')"
            :password-label="t('cta.demo_overlay.password')"
            :cancel-label="t('cta.demo_overlay.cancel')"
            :open-demo-label="t('cta.demo_overlay.open')"
            @update:model-value="showDemoDialog = $event"
        />
    </div>
</template>

<script setup lang="ts">
    import { ref, onMounted, onUnmounted } from 'vue'
    import Section from '@/components/Section.vue'
    import DemoCredentialsDialog from '@/components/DemoCredentialsDialog.vue'
    import { useTranslations } from '@/composables/useTranslations'
    import { useSEO } from '@/composables/useSEO'

    const { t } = useTranslations()
    const showDemoDialog = ref(false)

    useSEO({
        title: () => t('roadmap.seo.title'),
        description: () => t('roadmap.seo.description'),
    })

    // Array lengths for v-for loops
    const doneFeatures = Array(10).fill(null)
    const openFeatures = Array(6).fill(null)

    // Wish items with icons
    const wishItems = [{ icon: '💡' }, { icon: '🔧' }, { icon: '🚀' }]

    const openContact = () => {
        const email = t('contact.email')
        const subject = encodeURIComponent('Feature Request for RDataCore')
        window.open(`mailto:${email}?subject=${subject}`, '_blank', 'noopener,noreferrer')
    }

    // Listen for the open-demo event from Header
    const handleOpenDemo = () => {
        showDemoDialog.value = true
    }

    onMounted(() => {
        window.addEventListener('open-demo', handleOpenDemo)
    })

    onUnmounted(() => {
        window.removeEventListener('open-demo', handleOpenDemo)
    })
</script>

<style scoped>
    .roadmap-page {
        padding-top: 80px;
        min-height: 100vh;
    }

    .hero-section {
        padding: 80px 0 60px;
        background: linear-gradient(
            135deg,
            rgba(var(--v-theme-primary), 0.05) 0%,
            rgba(var(--v-theme-surface), 0) 100%
        );
    }

    .roadmap-container {
        max-width: 1200px;
        margin: 0 auto;
        padding: 0 24px;
    }

    .hero-content {
        text-align: center;
        max-width: 800px;
        margin: 0 auto;
    }

    .badge {
        display: inline-block;
        padding: 8px 16px;
        background: rgba(var(--v-theme-primary), 0.1);
        color: rgb(var(--v-theme-primary));
        border-radius: 20px;
        font-size: 0.875rem;
        font-weight: 600;
        margin-bottom: 24px;
    }

    h1 {
        font-size: clamp(2.5rem, 5vw, 3.5rem);
        font-weight: 700;
        margin: 0 0 24px;
        color: rgb(var(--v-theme-on-surface));
        line-height: 1.2;
    }

    .subtitle {
        font-size: 1.25rem;
        color: rgb(var(--v-theme-on-surface-variant));
        line-height: 1.6;
        margin: 0;
    }

    .features-section {
        padding: 60px 0;
    }

    .section-header {
        text-align: center;
        margin-bottom: 48px;
    }

    .section-header h2 {
        font-size: clamp(2rem, 4vw, 2.5rem);
        font-weight: 700;
        margin: 0 0 16px;
        color: rgb(var(--v-theme-on-surface));
    }

    .section-subtitle {
        font-size: 1.125rem;
        color: rgb(var(--v-theme-on-surface-variant));
        margin: 0;
    }

    .features-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
        gap: 24px;
    }

    .feature-card {
        background: rgba(var(--v-theme-surface), 1);
        border-radius: 16px;
        padding: 32px 24px;
        transition: all 0.3s ease;
        position: relative;
        overflow: hidden;
    }

    .feature-card.done {
        border: 2px solid rgb(var(--v-theme-success));
        box-shadow: 0 2px 8px rgba(var(--v-theme-success), 0.1);
    }

    .feature-card.open {
        border: 2px solid rgb(var(--v-theme-primary));
        box-shadow: 0 2px 8px rgba(var(--v-theme-primary), 0.1);
    }

    .feature-card:hover {
        transform: translateY(-4px);
        box-shadow: 0 8px 24px rgba(var(--v-theme-on-surface), 0.12);
    }

    .feature-card.done:hover {
        box-shadow: 0 8px 24px rgba(var(--v-theme-success), 0.2);
    }

    .feature-card.open:hover {
        box-shadow: 0 8px 24px rgba(var(--v-theme-primary), 0.2);
    }

    .feature-icon {
        width: 48px;
        height: 48px;
        border-radius: 12px;
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 24px;
        font-weight: 700;
        margin-bottom: 16px;
    }

    .feature-card.done .feature-icon {
        background: rgba(var(--v-theme-success), 0.1);
        color: rgb(var(--v-theme-success));
    }

    .feature-card.open .feature-icon {
        background: rgba(var(--v-theme-primary), 0.1);
        color: rgb(var(--v-theme-primary));
    }

    .feature-card h3 {
        font-size: 1.25rem;
        font-weight: 600;
        margin: 0 0 12px;
        color: rgb(var(--v-theme-on-surface));
    }

    .feature-card p {
        font-size: 0.95rem;
        color: rgb(var(--v-theme-on-surface-variant));
        line-height: 1.6;
        margin: 0;
    }

    /* Wish a Feature Section */
    .wish-section {
        padding: 80px 0;
        background: linear-gradient(
            180deg,
            rgba(var(--v-theme-surface-variant), 0.2) 0%,
            rgba(var(--v-theme-surface), 1) 100%
        );
    }

    .wish-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
        gap: 24px;
    }

    .wish-card {
        background: rgba(var(--v-theme-surface), 1);
        border-radius: 16px;
        padding: 32px 24px;
        transition: all 0.3s ease;
        position: relative;
        overflow: hidden;
        border: 2px dashed rgba(var(--v-theme-primary), 0.4);
        cursor: pointer;
        text-align: center;
    }

    .wish-card:hover {
        transform: translateY(-4px);
        border-color: rgb(var(--v-theme-primary));
        box-shadow: 0 8px 24px rgba(var(--v-theme-primary), 0.2);
    }

    .wish-icon {
        font-size: 48px;
        margin-bottom: 16px;
    }

    .wish-card h3 {
        font-size: 1.25rem;
        font-weight: 600;
        margin: 0 0 12px;
        color: rgb(var(--v-theme-on-surface));
    }

    .wish-card p {
        font-size: 0.95rem;
        color: rgb(var(--v-theme-on-surface-variant));
        line-height: 1.6;
        margin: 0 0 20px;
    }

    .wish-cta {
        display: inline-block;
        padding: 10px 24px;
        background: rgba(var(--v-theme-primary), 0.1);
        color: rgb(var(--v-theme-primary));
        border-radius: 20px;
        font-size: 0.875rem;
        font-weight: 600;
        transition: all 0.2s ease;
    }

    .wish-card:hover .wish-cta {
        background: rgb(var(--v-theme-primary));
        color: white;
    }

    @media (max-width: 960px) {
        .roadmap-page {
            padding-top: 60px;
        }

        .hero-section {
            padding: 60px 0 40px;
        }

        .features-section {
            padding: 40px 0;
        }

        .features-grid {
            grid-template-columns: 1fr;
        }

        .wish-section {
            padding: 60px 0;
        }

        .wish-grid {
            grid-template-columns: 1fr;
        }
    }
</style>
