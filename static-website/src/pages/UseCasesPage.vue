<template>
    <div class="use-cases-page">
        <Section class="hero-section">
            <div class="use-cases-container">
                <div class="hero-content">
                    <div class="badge">{{ t('use_cases.hero.badge') }}</div>
                    <h1>{{ t('use_cases.hero.title') }}</h1>
                    <p class="subtitle">{{ t('use_cases.hero.subtitle') }}</p>
                </div>
            </div>
        </Section>

        <Section class="intro-section">
            <div class="use-cases-container">
                <div class="intro-content">
                    <h2>{{ t('use_cases.intro.title') }}</h2>
                    <p>{{ t('use_cases.intro.description') }}</p>
                    <div class="capabilities">
                        <div
                            v-for="(_capability, index) in 5"
                            :key="index"
                            class="capability-item"
                        >
                            <span class="capability-icon">✓</span>
                            {{ t(`use_cases.intro.capabilities.${index}`) }}
                        </div>
                    </div>
                </div>
            </div>
        </Section>

        <Section class="cases-section">
            <div class="use-cases-container">
                <div class="section-header">
                    <h2>{{ t('use_cases.cases.title') }}</h2>
                    <p class="section-subtitle">{{ t('use_cases.cases.subtitle') }}</p>
                </div>
                <div class="cases-grid">
                    <div
                        v-for="(useCase, index) in useCases"
                        :key="index"
                        class="use-case-card"
                    >
                        <div class="case-icon">{{ useCase.icon }}</div>
                        <h3>{{ t(`use_cases.cases.items.${index}.title`) }}</h3>
                        <p>{{ t(`use_cases.cases.items.${index}.description`) }}</p>
                        <div class="case-tags">
                            <span
                                v-for="(tag, tagIndex) in useCase.tags"
                                :key="tagIndex"
                                class="tag"
                            >
                                {{ tag }}
                            </span>
                        </div>
                    </div>
                </div>
            </div>
        </Section>

        <Section class="cta-section">
            <div class="use-cases-container">
                <div class="cta-content">
                    <h2>{{ t('use_cases.cta.title') }}</h2>
                    <p>{{ t('use_cases.cta.subtitle') }}</p>
                    <div class="cta-buttons">
                        <v-btn
                            color="primary"
                            size="large"
                            rounded
                            @click="openDemo"
                        >
                            {{ t('cta.try_demo') }}
                        </v-btn>
                        <v-btn
                            variant="outlined"
                            size="large"
                            rounded
                            :href="`mailto:${t('contact.email')}`"
                        >
                            {{ t('use_cases.cta.contact') }}
                        </v-btn>
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
        title: t('use_cases.seo.title'),
        description: t('use_cases.seo.description'),
    })

    const useCases = [
        { icon: '🎯', tags: ['E-commerce', 'Aggregation'] },
        { icon: '📊', tags: ['CRM', 'Data Collection'] },
        { icon: '🔌', tags: ['Public API', 'Partners'] },
        { icon: '🗄️', tags: ['MDM', 'Central Hub'] },
        { icon: '⚙️', tags: ['CSV', 'JSON', 'Transform'] },
    ]

    const openDemo = () => {
        showDemoDialog.value = true
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
    .use-cases-page {
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

    .use-cases-container {
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

    .intro-section {
        padding: 60px 0;
        background: rgba(var(--v-theme-surface-variant), 0.3);
    }

    .intro-content {
        max-width: 800px;
        margin: 0 auto;
        text-align: center;
    }

    .intro-content h2 {
        font-size: clamp(1.75rem, 4vw, 2.25rem);
        font-weight: 700;
        margin: 0 0 20px;
        color: rgb(var(--v-theme-on-surface));
    }

    .intro-content > p {
        font-size: 1.125rem;
        color: rgb(var(--v-theme-on-surface-variant));
        line-height: 1.7;
        margin: 0 0 32px;
    }

    .capabilities {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
        gap: 16px;
        margin-top: 32px;
    }

    .capability-item {
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 12px 16px;
        background: rgba(var(--v-theme-surface), 0.6);
        border-radius: 12px;
        font-size: 0.95rem;
        color: rgb(var(--v-theme-on-surface));
        border: 1px solid rgba(var(--v-theme-primary), 0.2);
    }

    .capability-icon {
        color: rgb(var(--v-theme-success));
        font-weight: 700;
        font-size: 1.1rem;
    }

    .cases-section {
        padding: 80px 0;
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

    .cases-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(340px, 1fr));
        gap: 32px;
    }

    .use-case-card {
        background: rgba(var(--v-theme-surface), 1);
        border-radius: 20px;
        padding: 40px 32px;
        border: 2px solid rgba(var(--v-theme-primary), 0.15);
        transition: all 0.3s ease;
        position: relative;
        overflow: hidden;
    }

    .use-case-card::before {
        content: '';
        position: absolute;
        top: 0;
        left: 0;
        right: 0;
        height: 4px;
        background: linear-gradient(
            90deg,
            rgb(var(--v-theme-primary)),
            rgb(var(--v-theme-secondary))
        );
        transform: scaleX(0);
        transition: transform 0.3s ease;
    }

    .use-case-card:hover {
        transform: translateY(-8px);
        box-shadow: 0 12px 32px rgba(var(--v-theme-on-surface), 0.15);
        border-color: rgb(var(--v-theme-primary));
    }

    .use-case-card:hover::before {
        transform: scaleX(1);
    }

    .case-icon {
        width: 64px;
        height: 64px;
        border-radius: 16px;
        background: linear-gradient(
            135deg,
            rgba(var(--v-theme-primary), 0.1),
            rgba(var(--v-theme-secondary), 0.1)
        );
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 32px;
        margin-bottom: 20px;
    }

    .use-case-card h3 {
        font-size: 1.5rem;
        font-weight: 600;
        margin: 0 0 16px;
        color: rgb(var(--v-theme-on-surface));
    }

    .use-case-card p {
        font-size: 1rem;
        color: rgb(var(--v-theme-on-surface-variant));
        line-height: 1.7;
        margin: 0 0 20px;
    }

    .case-tags {
        display: flex;
        flex-wrap: wrap;
        gap: 8px;
    }

    .tag {
        padding: 6px 12px;
        background: rgba(var(--v-theme-primary), 0.1);
        color: rgb(var(--v-theme-primary));
        border-radius: 8px;
        font-size: 0.8rem;
        font-weight: 500;
    }

    .cta-section {
        padding: 80px 0;
        background: linear-gradient(
            135deg,
            rgba(var(--v-theme-primary), 0.08) 0%,
            rgba(var(--v-theme-surface), 0) 100%
        );
    }

    .cta-content {
        text-align: center;
        max-width: 700px;
        margin: 0 auto;
    }

    .cta-content h2 {
        font-size: clamp(2rem, 4vw, 2.75rem);
        font-weight: 700;
        margin: 0 0 20px;
        color: rgb(var(--v-theme-on-surface));
    }

    .cta-content p {
        font-size: 1.125rem;
        color: rgb(var(--v-theme-on-surface-variant));
        line-height: 1.6;
        margin: 0 0 32px;
    }

    .cta-buttons {
        display: flex;
        gap: 16px;
        justify-content: center;
        flex-wrap: wrap;
    }

    @media (max-width: 960px) {
        .use-cases-page {
            padding-top: 60px;
        }

        .hero-section {
            padding: 60px 0 40px;
        }

        .intro-section {
            padding: 40px 0;
        }

        .cases-section {
            padding: 60px 0;
        }

        .cta-section {
            padding: 60px 0;
        }

        .cases-grid {
            grid-template-columns: 1fr;
        }

        .capabilities {
            grid-template-columns: 1fr;
        }
    }
</style>
