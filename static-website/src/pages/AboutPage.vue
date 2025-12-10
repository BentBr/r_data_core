<template>
    <div class="about-page">
        <!-- Hero Section -->
        <Section class="about-hero-section">
            <div class="about-hero-container">
                <div class="sloth-emoji-large">🦥</div>
                <div class="about-badge">{{ t('about.hero.badge') }}</div>
                <h1>{{ t('about.hero.title') }}</h1>
                <p class="about-subtitle">
                    {{ t('about.hero.subtitle') }}
                </p>
            </div>
        </Section>

        <!-- Our Story Section -->
        <Section class="story-section">
            <v-row align="center">
                <v-col
                    cols="12"
                    md="6"
                >
                    <h2>{{ t('about.story.title') }}</h2>
                    <p>
                        {{ t('about.story.paragraph1') }}
                    </p>
                    <p>
                        {{ t('about.story.paragraph2') }}
                    </p>
                    <p>
                        {{ t('about.story.paragraph3') }}
                    </p>
                </v-col>
                <v-col
                    cols="12"
                    md="6"
                >
                    <div class="story-image">
                        <picture>
                            <source
                                type="image/webp"
                                srcset="
                                    /images/Slothlike-400w.webp   400w,
                                    /images/Slothlike-800w.webp   800w,
                                    /images/Slothlike-1200w.webp 1200w,
                                    /images/Slothlike-1600w.webp 1600w
                                "
                                sizes="(max-width: 768px) 100vw, (max-width: 1200px) 50vw, 500px"
                            />
                            <img
                                src="/images/Slothlike.png"
                                :alt="t('about.story.image_alt')"
                                class="slothlike-image"
                                loading="lazy"
                            />
                        </picture>
                    </div>
                </v-col>
            </v-row>
        </Section>

        <!-- Our Values Section -->
        <Section class="values-section">
            <header class="section-heading">
                <h2>{{ t('about.values.title') }}</h2>
                <p class="section-subtitle">{{ t('about.values.subtitle') }}</p>
            </header>
            <v-row>
                <v-col
                    cols="12"
                    sm="6"
                    lg="3"
                >
                    <v-card
                        class="value-card"
                        elevation="0"
                    >
                        <v-card-text>
                            <div class="value-icon">
                                <SmartIcon
                                    icon="shield"
                                    color="primary"
                                    size="lg"
                                />
                            </div>
                            <h3>{{ t('about.values.security.title') }}</h3>
                            <p>
                                {{ t('about.values.security.desc') }}
                            </p>
                        </v-card-text>
                    </v-card>
                </v-col>
                <v-col
                    cols="12"
                    sm="6"
                    lg="3"
                >
                    <v-card
                        class="value-card"
                        elevation="0"
                    >
                        <v-card-text>
                            <div class="value-icon">
                                <SmartIcon
                                    icon="code-2"
                                    color="primary"
                                    size="lg"
                                />
                            </div>
                            <h3>{{ t('about.values.open_source.title') }}</h3>
                            <p>
                                {{ t('about.values.open_source.desc') }}
                            </p>
                        </v-card-text>
                    </v-card>
                </v-col>
                <v-col
                    cols="12"
                    sm="6"
                    lg="3"
                >
                    <v-card
                        class="value-card"
                        elevation="0"
                    >
                        <v-card-text>
                            <div class="value-icon">
                                <SmartIcon
                                    icon="zap"
                                    color="primary"
                                    size="lg"
                                />
                            </div>
                            <h3>{{ t('about.values.fast.title') }}</h3>
                            <p>{{ t('about.values.fast.desc') }}</p>
                        </v-card-text>
                    </v-card>
                </v-col>
                <v-col
                    cols="12"
                    sm="6"
                    lg="3"
                >
                    <v-card
                        class="value-card"
                        elevation="0"
                    >
                        <v-card-text>
                            <div class="value-icon">
                                <SmartIcon
                                    icon="coffee"
                                    color="primary"
                                    size="lg"
                                />
                            </div>
                            <h3>{{ t('about.values.developer_love.title') }}</h3>
                            <p>
                                {{ t('about.values.developer_love.desc') }}
                            </p>
                        </v-card-text>
                    </v-card>
                </v-col>
            </v-row>
        </Section>

        <!-- By the Numbers Section -->
        <Section class="numbers-section">
            <header class="section-heading">
                <h2>{{ t('about.numbers.title') }}</h2>
            </header>
            <v-row>
                <v-col
                    cols="12"
                    sm="6"
                    lg="3"
                >
                    <div class="number-stat">
                        <div class="number-value">{{ t('about.numbers.open_source.value') }}</div>
                        <div class="number-label">{{ t('about.numbers.open_source.label') }}</div>
                        <div class="number-desc">{{ t('about.numbers.open_source.desc') }}</div>
                    </div>
                </v-col>
                <v-col
                    cols="12"
                    sm="6"
                    lg="3"
                >
                    <div class="number-stat">
                        <div class="number-value">{{ t('about.numbers.coffee.value') }}</div>
                        <div class="number-label">{{ t('about.numbers.coffee.label') }}</div>
                        <div class="number-desc">{{ t('about.numbers.coffee.desc') }}</div>
                    </div>
                </v-col>
                <v-col
                    cols="12"
                    sm="6"
                    lg="3"
                >
                    <div class="number-stat">
                        <div class="number-value">{{ t('about.numbers.sloth.value') }}</div>
                        <div class="number-label">{{ t('about.numbers.sloth.label') }}</div>
                        <div class="number-desc">{{ t('about.numbers.sloth.desc') }}</div>
                    </div>
                </v-col>
                <v-col
                    cols="12"
                    sm="6"
                    lg="3"
                >
                    <div class="number-stat">
                        <div class="number-value">{{ t('about.numbers.rust.value') }}</div>
                        <div class="number-label">{{ t('about.numbers.rust.label') }}</div>
                        <div class="number-desc">{{ t('about.numbers.rust.desc') }}</div>
                    </div>
                </v-col>
            </v-row>
        </Section>

        <!-- Meet the Squad Section -->
        <Section class="squad-section">
            <header class="section-heading">
                <h2>{{ t('about.squad.title') }}</h2>
                <p class="section-subtitle">
                    {{ t('about.squad.subtitle') }}
                </p>
            </header>
        </Section>

        <!-- Tech Stack Section -->
        <Section class="tech-stack-section">
            <header class="section-heading">
                <div class="section-badge">{{ t('about.tech_stack.badge') }}</div>
                <h2>{{ t('about.tech_stack.title') }}</h2>
                <p class="section-subtitle">
                    {{ t('about.tech_stack.subtitle') }}
                </p>
            </header>
            <v-row>
                <v-col
                    cols="12"
                    lg="6"
                >
                    <v-card
                        class="tech-card"
                        elevation="0"
                    >
                        <v-card-text>
                            <div class="tech-header">
                                <SmartIcon
                                    icon="database"
                                    color="primary"
                                    size="lg"
                                />
                                <h3>{{ t('about.tech_stack.backend.title') }}</h3>
                            </div>
                            <div class="tech-list">
                                <div class="tech-item">
                                    <span class="tech-name">{{
                                        t('about.tech_stack.backend.rust.name')
                                    }}</span>
                                    <span class="tech-desc">
                                        {{ t('about.tech_stack.backend.rust.desc') }}
                                    </span>
                                </div>
                                <div class="tech-item">
                                    <span class="tech-name">{{
                                        t('about.tech_stack.backend.actix.name')
                                    }}</span>
                                    <span class="tech-desc">
                                        {{ t('about.tech_stack.backend.actix.desc') }}
                                    </span>
                                </div>
                                <div class="tech-item">
                                    <span class="tech-name">{{
                                        t('about.tech_stack.backend.postgresql.name')
                                    }}</span>
                                    <span class="tech-desc">
                                        {{ t('about.tech_stack.backend.postgresql.desc') }}
                                    </span>
                                </div>
                                <div class="tech-item">
                                    <span class="tech-name">{{
                                        t('about.tech_stack.backend.redis.name')
                                    }}</span>
                                    <span class="tech-desc">
                                        {{ t('about.tech_stack.backend.redis.desc') }}
                                    </span>
                                </div>
                                <div class="tech-item">
                                    <span class="tech-name">{{
                                        t('about.tech_stack.backend.openapi.name')
                                    }}</span>
                                    <span class="tech-desc">
                                        {{ t('about.tech_stack.backend.openapi.desc') }}
                                    </span>
                                </div>
                            </div>
                        </v-card-text>
                    </v-card>
                </v-col>
                <v-col
                    cols="12"
                    lg="6"
                >
                    <v-card
                        class="tech-card highlighted"
                        elevation="2"
                    >
                        <v-card-text>
                            <div class="tech-header">
                                <SmartIcon
                                    icon="code-2"
                                    color="primary"
                                    size="lg"
                                />
                                <h3>{{ t('about.tech_stack.frontend.title') }}</h3>
                            </div>
                            <div class="tech-list">
                                <div class="tech-item">
                                    <span class="tech-name">{{
                                        t('about.tech_stack.frontend.vue.name')
                                    }}</span>
                                    <span class="tech-desc">
                                        {{ t('about.tech_stack.frontend.vue.desc') }}
                                    </span>
                                </div>
                                <div class="tech-item">
                                    <span class="tech-name">{{
                                        t('about.tech_stack.frontend.pinia.name')
                                    }}</span>
                                    <span class="tech-desc">
                                        {{ t('about.tech_stack.frontend.pinia.desc') }}
                                    </span>
                                </div>
                                <div class="tech-item">
                                    <span class="tech-name">{{
                                        t('about.tech_stack.frontend.router.name')
                                    }}</span>
                                    <span class="tech-desc">
                                        {{ t('about.tech_stack.frontend.router.desc') }}
                                    </span>
                                </div>
                                <div class="tech-item">
                                    <span class="tech-name">{{
                                        t('about.tech_stack.frontend.vuetify.name')
                                    }}</span>
                                    <span class="tech-desc">
                                        {{ t('about.tech_stack.frontend.vuetify.desc') }}
                                    </span>
                                </div>
                            </div>
                        </v-card-text>
                    </v-card>
                </v-col>
            </v-row>
        </Section>

        <!-- Architecture Section -->
        <Section class="architecture-section">
            <v-card
                class="architecture-card"
                elevation="2"
            >
                <v-card-text>
                    <div class="arch-header">
                        <SmartIcon
                            icon="layers"
                            color="primary"
                            size="lg"
                        />
                        <h3>{{ t('about.architecture.title') }}</h3>
                    </div>
                    <v-row class="arch-grid">
                        <v-col
                            cols="6"
                            md="3"
                        >
                            <div class="arch-item">
                                <SmartIcon
                                    icon="database"
                                    color="primary"
                                    size="xl"
                                />
                                <h4>{{ t('about.architecture.postgresql.title') }}</h4>
                                <p>{{ t('about.architecture.postgresql.desc') }}</p>
                            </div>
                        </v-col>
                        <v-col
                            cols="6"
                            md="3"
                        >
                            <div class="arch-item">
                                <SmartIcon
                                    icon="server"
                                    color="primary"
                                    size="xl"
                                />
                                <h4>{{ t('about.architecture.backend.title') }}</h4>
                                <p>{{ t('about.architecture.backend.desc') }}</p>
                            </div>
                        </v-col>
                        <v-col
                            cols="6"
                            md="3"
                        >
                            <div class="arch-item">
                                <SmartIcon
                                    icon="boxes"
                                    color="primary"
                                    size="xl"
                                />
                                <h4>{{ t('about.architecture.workers.title') }}</h4>
                                <p>{{ t('about.architecture.workers.desc') }}</p>
                            </div>
                        </v-col>
                        <v-col
                            cols="6"
                            md="3"
                        >
                            <div class="arch-item">
                                <SmartIcon
                                    icon="zap"
                                    color="primary"
                                    size="xl"
                                />
                                <h4>{{ t('about.architecture.redis.title') }}</h4>
                                <p>{{ t('about.architecture.redis.desc') }}</p>
                            </div>
                        </v-col>
                    </v-row>
                    <div class="arch-callout">
                        <span class="callout-highlight">{{
                            t('about.architecture.callout_highlight')
                        }}</span>
                        — {{ t('about.architecture.callout') }}
                    </div>
                </v-card-text>
            </v-card>
        </Section>

        <!-- Mission Section -->
        <Section class="mission-section">
            <v-card
                class="mission-card"
                elevation="8"
            >
                <v-card-text>
                    <div class="mission-icon">🎯</div>
                    <h2>{{ t('about.mission.title') }}</h2>
                    <p>
                        {{ t('about.mission.text') }}
                    </p>
                </v-card-text>
            </v-card>
        </Section>

        <!-- Sloth Image Section -->
        <Section class="sloth-image-section">
            <div class="sloth-image-container">
                <picture>
                    <source
                        type="image/webp"
                        srcset="
                            /images/Active-sloth-400w.webp   400w,
                            /images/Active-sloth-800w.webp   800w,
                            /images/Active-sloth-1200w.webp 1200w,
                            /images/Active-sloth-1600w.webp 1600w
                        "
                        sizes="(max-width: 768px) 100vw, (max-width: 1200px) 80vw, 800px"
                    />
                    <img
                        src="/images/Active-sloth.png"
                        alt="Slothlike Mascot - Slow & Steady"
                        class="active-sloth-image"
                        loading="lazy"
                    />
                </picture>
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
    import SmartIcon from '@/components/common/SmartIcon.vue'
    import DemoCredentialsDialog from '@/components/DemoCredentialsDialog.vue'
    import { useSEO } from '@/composables/useSEO'
    import { useTranslations } from '@/composables/useTranslations'

    const { t } = useTranslations()
    const showDemoDialog = ref(false)

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

    useSEO({
        title: t('seo.about.title'),
        description: t('seo.about.description'),
    })
</script>

<style scoped>
    .about-page {
        padding-top: 80px;
    }

    /* Hero Section */
    .about-hero-section {
        padding: 80px 0;
        text-align: center;
        background: linear-gradient(
            180deg,
            rgba(var(--v-theme-surface), 1) 0%,
            rgba(var(--v-theme-surface-variant), 0.3) 100%
        );
    }

    .about-hero-container {
        max-width: 800px;
        margin: 0 auto;
    }

    .sloth-emoji-large {
        font-size: 4rem;
        margin-bottom: 16px;
    }

    .about-badge {
        display: inline-block;
        background: rgb(var(--v-theme-primary));
        color: white;
        padding: 8px 20px;
        border-radius: 24px;
        font-size: 0.875rem;
        font-weight: 600;
        margin-bottom: 24px;
    }

    .about-hero-section h1 {
        font-size: clamp(2.5rem, 5vw, 4rem);
        font-weight: 800;
        line-height: 1.1;
        margin: 0 0 24px;
    }

    .about-subtitle {
        font-size: 1.25rem;
        line-height: 1.6;
        color: rgb(var(--v-theme-on-surface-variant));
        margin: 0;
    }

    /* Story Section */
    .story-section {
        padding: 80px 0;
    }

    .story-section h2 {
        font-size: 2.5rem;
        font-weight: 700;
        margin: 0 0 24px;
    }

    .story-section p {
        font-size: 1.125rem;
        line-height: 1.7;
        color: rgb(var(--v-theme-on-surface-variant));
        margin-bottom: 20px;
    }

    .story-image {
        display: flex;
        align-items: center;
        justify-content: center;
        min-height: 300px;
    }

    .slothlike-image {
        width: 100%;
        max-width: 500px;
        height: auto;
        border-radius: 16px;
        box-shadow: 0 4px 16px rgba(0, 0, 0, 0.1);
    }

    /* Values Section */
    .values-section {
        padding: 80px 0;
        background: rgba(var(--v-theme-surface-variant), 0.2);
    }

    .section-heading {
        text-align: center;
        margin-bottom: 60px;
    }

    .section-heading h2 {
        font-size: clamp(2rem, 4vw, 3rem);
        font-weight: 700;
        margin: 0 0 16px;
    }

    .section-subtitle {
        font-size: 1.125rem;
        color: rgb(var(--v-theme-on-surface-variant));
        max-width: 700px;
        margin: 0 auto;
        line-height: 1.6;
    }

    .section-badge {
        display: inline-block;
        background: rgb(var(--v-theme-primary));
        color: white;
        padding: 6px 16px;
        border-radius: 20px;
        font-size: 0.875rem;
        font-weight: 600;
        margin-bottom: 16px;
    }

    .value-card {
        height: 100%;
        border: 1px solid rgba(var(--v-theme-on-surface), 0.12);
        text-align: center;
    }

    .value-icon {
        margin-bottom: 16px;
        padding: 16px;
        background: rgba(var(--v-theme-primary), 0.1);
        border-radius: 50%;
        display: inline-flex;
    }

    .value-card h3 {
        font-size: 1.25rem;
        font-weight: 700;
        margin-bottom: 12px;
    }

    .value-card p {
        color: rgb(var(--v-theme-on-surface-variant));
        line-height: 1.6;
        margin: 0;
    }

    /* Numbers Section */
    .numbers-section {
        padding: 80px 0;
    }

    .number-stat {
        text-align: center;
        padding: 32px 16px;
    }

    .number-value {
        font-size: 4rem;
        font-weight: 800;
        margin-bottom: 12px;
        color: rgb(var(--v-theme-primary));
    }

    .number-label {
        font-size: 1.25rem;
        font-weight: 700;
        margin-bottom: 8px;
        color: rgb(var(--v-theme-on-surface));
    }

    .number-desc {
        font-size: 1rem;
        color: rgb(var(--v-theme-on-surface-variant));
    }

    /* Squad Section */
    .squad-section {
        padding: 80px 0;
        background: rgba(var(--v-theme-surface-variant), 0.2);
    }

    /* Tech Stack Section */
    .tech-stack-section {
        padding: 80px 0;
    }

    .tech-card {
        border: 1px solid rgba(var(--v-theme-on-surface), 0.12);
        padding: 24px;
        height: 100%;
    }

    .tech-card.highlighted {
        border: 2px solid rgb(var(--v-theme-primary));
    }

    .tech-header {
        display: flex;
        align-items: center;
        gap: 12px;
        margin-bottom: 24px;
    }

    .tech-header h3 {
        font-size: 1.75rem;
        font-weight: 700;
        margin: 0;
    }

    .tech-list {
        display: grid;
        gap: 20px;
    }

    .tech-item {
        display: grid;
        gap: 6px;
    }

    .tech-name {
        font-weight: 700;
        color: rgb(var(--v-theme-primary));
        font-size: 1.125rem;
    }

    .tech-desc {
        color: rgb(var(--v-theme-on-surface-variant));
        line-height: 1.5;
    }

    /* Architecture Section */
    .architecture-section {
        padding: 80px 0;
        background: rgba(var(--v-theme-surface-variant), 0.1);
    }

    .architecture-card {
        border: 1px solid rgba(var(--v-theme-primary), 0.3);
        padding: 32px;
        max-width: 1000px;
        margin: 0 auto;
    }

    .arch-header {
        display: flex;
        align-items: center;
        gap: 12px;
        margin-bottom: 40px;
        justify-content: center;
    }

    .arch-header h3 {
        font-size: 2rem;
        font-weight: 700;
        margin: 0;
    }

    .arch-grid {
        margin-bottom: 32px;
    }

    .arch-item {
        text-align: center;
        padding: 24px 16px;
    }

    .arch-item h4 {
        font-size: 1.125rem;
        font-weight: 700;
        margin: 12px 0 6px;
    }

    .arch-item p {
        color: rgb(var(--v-theme-on-surface-variant));
        margin: 0;
        font-size: 0.9rem;
    }

    .arch-callout {
        text-align: center;
        padding: 24px;
        background: rgba(var(--v-theme-primary), 0.05);
        border-radius: 12px;
        border: 1px solid rgba(var(--v-theme-primary), 0.2);
        font-size: 1.125rem;
        line-height: 1.6;
    }

    .callout-highlight {
        font-weight: 700;
        color: rgb(var(--v-theme-primary));
    }

    /* Mission Section */
    .mission-section {
        padding: 80px 0 40px;
    }

    .mission-card {
        background: linear-gradient(135deg, rgb(var(--v-theme-primary)) 0%, #e85d04 100%);
        color: white;
        text-align: center;
        padding: 60px 40px;
        border-radius: 24px;
        max-width: 900px;
        margin: 0 auto;
    }

    .mission-icon {
        font-size: 4rem;
        margin-bottom: 24px;
    }

    .mission-card h2 {
        font-size: clamp(2rem, 4vw, 3rem);
        font-weight: 700;
        margin: 0 0 24px;
        color: white;
    }

    .mission-card p {
        font-size: 1.375rem;
        line-height: 1.7;
        margin: 0;
        opacity: 0.95;
    }

    /* Sloth Image Section */
    .sloth-image-section {
        padding: 40px 0 80px;
    }

    .sloth-image-container {
        max-width: 800px;
        margin: 0 auto;
        text-align: center;
    }

    .active-sloth-image {
        width: 100%;
        max-width: 600px;
        height: auto;
        border-radius: 24px;
        box-shadow: 0 8px 32px rgba(0, 0, 0, 0.15);
    }

    @media (max-width: 960px) {
        .about-page {
            padding-top: 60px;
        }

        .about-hero-section,
        .story-section,
        .values-section,
        .numbers-section,
        .squad-section,
        .tech-stack-section,
        .architecture-section,
        .mission-section {
            padding: 60px 0;
        }
    }
</style>
