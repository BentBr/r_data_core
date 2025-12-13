<template>
    <div class="home-page">
        <!-- Hero Section -->
        <Section class="hero-section">
            <div class="hero-container">
                <div class="hero-badge">
                    <span class="sloth-emoji">🦥</span>
                    {{ t('home.hero.badge') }}
                </div>
                <h1 class="hero-title">
                    {{ t('home.hero.title_line1') }}<br />{{ t('home.hero.title_line2') }}
                    <span class="sloth-icon">🦥</span>
                </h1>
                <p class="hero-subtitle">
                    {{ t('home.hero.subtitle') }}
                </p>
                <div class="hero-actions">
                    <v-btn
                        color="primary"
                        size="x-large"
                        rounded
                        elevation="2"
                        @click="() => $router.push('/pricing')"
                    >
                        {{ t('home.hero.get_started') }}
                        <SmartIcon
                            icon="arrow-right"
                            size="sm"
                        />
                    </v-btn>
                    <v-btn
                        variant="outlined"
                        size="x-large"
                        rounded
                        @click="openDemo"
                    >
                        {{ t('home.hero.try_demo') }}
                    </v-btn>
                </div>
                <p class="hero-footnote">
                    {{ t('home.hero.footnote') }}
                </p>
                <div class="hero-links">
                    <a
                        v-if="apiDocsUrl"
                        :href="apiDocsUrl"
                        target="_blank"
                        rel="noopener noreferrer"
                        class="doc-link"
                    >
                        {{ t('home.hero.api_docs') }}
                    </a>
                    <a
                        v-if="adminApiDocsUrl"
                        :href="adminApiDocsUrl"
                        target="_blank"
                        rel="noopener noreferrer"
                        class="doc-link"
                    >
                        {{ t('home.hero.admin_api_docs') }}
                    </a>
                    <a
                        v-if="githubReleasesUrl"
                        :href="githubReleasesUrl"
                        target="_blank"
                        rel="noopener noreferrer"
                        class="doc-link"
                    >
                        {{ t('home.hero.github_releases') }}
                    </a>
                </div>
            </div>
        </Section>

        <!-- Built Different Section -->
        <Section class="built-different-section">
            <div class="built-different-container">
                <v-row>
                    <v-col
                        cols="12"
                        sm="6"
                        lg="3"
                    >
                        <div class="stat-card">
                            <div class="stat-value">
                                {{ t('home.built_different.self_hosted.value') }}
                            </div>
                            <div class="stat-label">
                                {{ t('home.built_different.self_hosted.label') }}
                            </div>
                            <div class="stat-desc">
                                {{ t('home.built_different.self_hosted.desc') }}
                            </div>
                        </div>
                    </v-col>
                    <v-col
                        cols="12"
                        sm="6"
                        lg="3"
                    >
                        <div class="stat-card">
                            <div class="stat-value">{{ t('home.built_different.rust.value') }}</div>
                            <div class="stat-label">{{ t('home.built_different.rust.label') }}</div>
                            <div class="stat-desc">{{ t('home.built_different.rust.desc') }}</div>
                        </div>
                    </v-col>
                    <v-col
                        cols="12"
                        sm="6"
                        lg="3"
                    >
                        <div class="stat-card">
                            <div class="stat-value">{{ t('home.built_different.free.value') }}</div>
                            <div class="stat-label">{{ t('home.built_different.free.label') }}</div>
                            <div class="stat-desc">{{ t('home.built_different.free.desc') }}</div>
                        </div>
                    </v-col>
                    <v-col
                        cols="12"
                        sm="6"
                        lg="3"
                    >
                        <div class="stat-card">
                            <div class="stat-value">{{ t('home.built_different.open.value') }}</div>
                            <div class="stat-label">{{ t('home.built_different.open.label') }}</div>
                            <div class="stat-desc">{{ t('home.built_different.open.desc') }}</div>
                        </div>
                    </v-col>
                </v-row>
            </div>
        </Section>

        <!-- Everything You Need Section -->
        <Section
            id="features"
            class="features-section"
        >
            <div class="page-container">
                <header class="section-heading">
                    <h2>{{ t('home.features.title') }}</h2>
                    <p class="section-subtitle">
                        {{ t('home.features.subtitle') }}
                    </p>
                </header>
                <v-row>
                    <v-col
                        v-for="feature in featureItems"
                        :key="feature.title"
                        cols="12"
                        md="6"
                        lg="4"
                    >
                        <FeatureCard
                            :icon="feature.icon"
                            :title="feature.title"
                            :description="feature.description"
                        />
                    </v-col>
                </v-row>
            </div>
        </Section>

        <!-- Data Flows Diagram Section -->
        <Section class="data-flows-section">
            <header class="section-heading">
                <h2>{{ t('home.data_flows.title') }}</h2>
                <p class="section-subtitle">
                    {{ t('home.data_flows.subtitle') }}
                </p>
            </header>
            <div class="data-flows-container">
                <v-row align="center">
                    <v-col
                        cols="12"
                        md="3"
                    >
                        <div class="flow-column">
                            <h3>{{ t('home.data_flows.source_systems.title') }}</h3>
                            <ul>
                                <li
                                    v-for="(item, index) in get(
                                        'home.data_flows.source_systems.items'
                                    )"
                                    :key="index"
                                >
                                    {{ item }}
                                </li>
                            </ul>
                        </div>
                    </v-col>
                    <v-col
                        cols="12"
                        md="6"
                    >
                        <v-card
                            class="rdatacore-card"
                            elevation="4"
                        >
                            <v-card-text>
                                <div class="rdatacore-icon">
                                    <SmartIcon
                                        icon="database"
                                        color="primary"
                                        size="xl"
                                    />
                                </div>
                                <h3>{{ t('home.data_flows.rdatacore.title') }}</h3>
                                <ul class="rdatacore-features">
                                    <li
                                        v-for="(feature, index) in get(
                                            'home.data_flows.rdatacore.features'
                                        )"
                                        :key="index"
                                    >
                                        {{ feature }}
                                    </li>
                                </ul>
                            </v-card-text>
                        </v-card>
                    </v-col>
                    <v-col
                        cols="12"
                        md="3"
                    >
                        <div class="flow-column">
                            <h3>{{ t('home.data_flows.target_systems.title') }}</h3>
                            <ul>
                                <li
                                    v-for="(item, index) in get(
                                        'home.data_flows.target_systems.items'
                                    )"
                                    :key="index"
                                >
                                    {{ item }}
                                </li>
                            </ul>
                        </div>
                    </v-col>
                </v-row>
                <div class="flow-footnote">
                    {{ t('home.data_flows.footnote') }}
                </div>
            </div>
        </Section>

        <!-- Flexible Section -->
        <Section class="flexible-section">
            <div class="page-container">
                <header class="section-heading">
                    <div class="section-badge">{{ t('home.flexible.badge') }}</div>
                    <h2>{{ t('home.flexible.title') }}</h2>
                    <p class="section-subtitle">
                        {{ t('home.flexible.subtitle') }}
                    </p>
                </header>
                <v-row>
                    <v-col
                        cols="12"
                        md="6"
                        lg="4"
                    >
                        <v-card
                            class="feature-detail-card"
                            elevation="0"
                        >
                            <v-card-text>
                                <SmartIcon
                                    icon="git-branch"
                                    color="primary"
                                    size="lg"
                                />
                                <h3>{{ t('home.flexible.workflow.title') }}</h3>
                                <p>
                                    {{ t('home.flexible.workflow.desc') }}
                                </p>
                            </v-card-text>
                        </v-card>
                    </v-col>
                    <v-col
                        cols="12"
                        md="6"
                        lg="4"
                    >
                        <v-card
                            class="feature-detail-card"
                            elevation="0"
                        >
                            <v-card-text>
                                <SmartIcon
                                    icon="file-input"
                                    color="primary"
                                    size="lg"
                                />
                                <h3>{{ t('home.flexible.import_export.title') }}</h3>
                                <p>
                                    {{ t('home.flexible.import_export.desc') }}
                                </p>
                            </v-card-text>
                        </v-card>
                    </v-col>
                    <v-col
                        cols="12"
                        md="6"
                        lg="4"
                    >
                        <v-card
                            class="feature-detail-card"
                            elevation="0"
                        >
                            <v-card-text>
                                <SmartIcon
                                    icon="code-2"
                                    color="primary"
                                    size="lg"
                                />
                                <h3>{{ t('home.flexible.custom_api.title') }}</h3>
                                <p>
                                    {{ t('home.flexible.custom_api.desc') }}
                                </p>
                            </v-card-text>
                        </v-card>
                    </v-col>
                    <v-col
                        cols="12"
                        md="6"
                        lg="4"
                    >
                        <v-card
                            class="feature-detail-card"
                            elevation="0"
                        >
                            <v-card-text>
                                <SmartIcon
                                    icon="key"
                                    color="primary"
                                    size="lg"
                                />
                                <h3>{{ t('home.flexible.auth.title') }}</h3>
                                <p>
                                    {{ t('home.flexible.auth.desc') }}
                                </p>
                            </v-card-text>
                        </v-card>
                    </v-col>
                    <v-col
                        cols="12"
                        md="6"
                        lg="4"
                    >
                        <v-card
                            class="feature-detail-card highlighted"
                            elevation="2"
                        >
                            <v-card-text>
                                <SmartIcon
                                    icon="link"
                                    color="primary"
                                    size="lg"
                                />
                                <h3>{{ t('home.flexible.integrations.title') }}</h3>
                                <p>
                                    {{ t('home.flexible.integrations.desc') }}
                                </p>
                            </v-card-text>
                        </v-card>
                    </v-col>
                    <v-col
                        cols="12"
                        md="6"
                        lg="4"
                    >
                        <v-card
                            class="feature-detail-card"
                            elevation="0"
                        >
                            <v-card-text>
                                <SmartIcon
                                    icon="zap"
                                    color="primary"
                                    size="lg"
                                />
                                <h3>{{ t('home.flexible.performance.title') }}</h3>
                                <p>
                                    {{ t('home.flexible.performance.desc') }}
                                </p>
                            </v-card-text>
                        </v-card>
                    </v-col>
                </v-row>
            </div>
        </Section>

        <!-- Final CTA Section -->
        <Section class="final-cta-section">
            <v-card
                class="cta-card"
                elevation="8"
            >
                <v-card-text>
                    <div class="sloth-emoji-large">🦥</div>
                    <h2>{{ t('home.final_cta.title') }}</h2>
                    <p>
                        {{ t('home.final_cta.subtitle') }}
                    </p>
                    <div class="cta-actions">
                        <v-btn
                            color="white"
                            size="x-large"
                            rounded
                            elevation="0"
                            @click="() => $router.push('/pricing')"
                        >
                            {{ t('home.final_cta.view_pricing') }}
                        </v-btn>
                        <v-btn
                            variant="outlined"
                            color="white"
                            size="x-large"
                            rounded
                            @click="openContact"
                        >
                            {{ t('home.final_cta.contact_us') }}
                        </v-btn>
                    </div>
                </v-card-text>
            </v-card>
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
            @open-demo="trackDemoOpen"
        />
    </div>
</template>

<script setup lang="ts">
    import { ref, computed, onMounted, onUnmounted } from 'vue'
    import Section from '@/components/Section.vue'
    import FeatureCard from '@/components/FeatureCard.vue'
    import DemoCredentialsDialog from '@/components/DemoCredentialsDialog.vue'
    import SmartIcon from '@/components/common/SmartIcon.vue'
    import { useTranslations } from '@/composables/useTranslations'
    import { useSEO } from '@/composables/useSEO'
    import { env } from '@/env-check'

    const { t, get } = useTranslations()
    const showDemoDialog = ref(false)

    const featureItems = computed(() => get<Array<Record<string, string>>>('features.items') ?? [])

    // API documentation and releases URLs from environment
    const apiDocsUrl = env.apiDocsUrl
    const adminApiDocsUrl = env.adminApiDocsUrl
    const githubReleasesUrl = env.githubReleasesUrl

    const openDemo = () => {
        showDemoDialog.value = true
    }

    const openContact = () => {
        const email = t('contact.email')
        window.open(`mailto:${email}`, '_blank', 'noopener,noreferrer')
    }

    const trackDemoOpen = () => {
        // placeholder for analytics hook
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

    useSEO({
        title: t('seo.home.title'),
        description: t('seo.home.description'),
    })
</script>

<style scoped>
    .home-page {
        padding-top: 80px; /* Account for fixed header */
    }

    .page-container {
        max-width: 1200px;
        margin: 0 auto;
        padding: 0 8px;
    }

    /* Hero Section */
    .hero-section {
        background: linear-gradient(
            180deg,
            rgba(var(--v-theme-surface), 1) 0%,
            rgba(var(--v-theme-surface-variant), 0.3) 100%
        );
        padding: 80px 0;
    }

    .hero-container {
        max-width: 900px;
        margin: 0 auto;
        text-align: center;
    }

    .hero-badge {
        display: inline-flex;
        align-items: center;
        gap: 8px;
        background: rgb(var(--v-theme-primary));
        color: white;
        padding: 8px 20px;
        border-radius: 24px;
        font-size: 0.875rem;
        font-weight: 600;
        margin-bottom: 24px;
    }

    .sloth-emoji {
        font-size: 1.2em;
    }

    .hero-title {
        font-size: clamp(2.5rem, 5vw, 4rem);
        font-weight: 800;
        line-height: 1.1;
        margin: 0 0 24px;
        color: rgb(var(--v-theme-on-surface));
    }

    .sloth-icon {
        font-size: 0.9em;
    }

    .hero-subtitle {
        font-size: clamp(1.125rem, 2vw, 1.375rem);
        line-height: 1.6;
        color: rgb(var(--v-theme-on-surface-variant));
        margin: 0 0 40px;
        max-width: 800px;
        margin-left: auto;
        margin-right: auto;
    }

    .hero-actions {
        display: flex;
        gap: 16px;
        justify-content: center;
        flex-wrap: wrap;
        margin-bottom: 24px;
    }

    .hero-footnote {
        color: rgb(var(--v-theme-on-surface-variant));
        font-size: 1rem;
        margin: 0 0 16px;
    }

    .hero-links {
        display: flex;
        gap: 24px;
        justify-content: center;
        flex-wrap: wrap;
    }

    .doc-link {
        color: rgb(var(--v-theme-primary));
        text-decoration: none;
        font-weight: 600;
        font-size: 0.95rem;
        transition: opacity 0.2s ease;
    }

    .doc-link:hover {
        opacity: 0.7;
        text-decoration: underline;
    }

    /* Built Different Section */
    .built-different-section {
        padding: 60px 0;
    }

    .stat-card {
        text-align: center;
        padding: 32px 16px;
    }

    .stat-value {
        font-size: 3rem;
        font-weight: 800;
        margin-bottom: 8px;
        color: rgb(var(--v-theme-primary));
    }

    .stat-label {
        font-size: 1.5rem;
        font-weight: 700;
        margin-bottom: 8px;
        color: rgb(var(--v-theme-on-surface));
    }

    .stat-desc {
        font-size: 1rem;
        color: rgb(var(--v-theme-on-surface-variant));
    }

    /* Features Section */
    .features-section {
        background: rgba(var(--v-theme-surface-variant), 0.3);
        padding: 80px 0;
    }

    .section-heading {
        text-align: center;
        margin-bottom: 60px;
    }

    .section-heading h2 {
        font-size: clamp(2rem, 4vw, 3rem);
        font-weight: 700;
        margin: 0 0 16px;
        color: rgb(var(--v-theme-on-surface));
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

    /* Data Flows Section */
    .data-flows-section {
        padding: 80px 0;
    }

    .data-flows-container {
        max-width: 1200px;
        margin: 0 auto;
    }

    .flow-column h3 {
        font-size: 1.25rem;
        font-weight: 700;
        margin-bottom: 16px;
    }

    .flow-column ul {
        list-style: none;
        padding: 0;
        margin: 0;
    }

    .flow-column li {
        padding: 8px 0;
        color: rgb(var(--v-theme-on-surface-variant));
    }

    .rdatacore-card {
        border: 2px solid rgb(var(--v-theme-primary));
        text-align: center;
        padding: 24px;
    }

    .rdatacore-icon {
        margin-bottom: 16px;
    }

    .rdatacore-card h3 {
        font-size: 1.75rem;
        font-weight: 700;
        margin-bottom: 16px;
    }

    .rdatacore-features {
        list-style: none;
        padding: 0;
        margin: 0;
        text-align: left;
    }

    .rdatacore-features li {
        padding: 8px 0;
        font-size: 1.125rem;
    }

    .flow-footnote {
        text-align: center;
        margin-top: 40px;
        padding: 16px;
        background: rgb(var(--v-theme-primary));
        color: white;
        border-radius: 12px;
        font-size: 1.125rem;
        font-weight: 600;
    }

    /* Flexible Section */
    .flexible-section {
        background: rgba(var(--v-theme-surface-variant), 0.2);
        padding: 80px 0;
    }

    .feature-detail-card {
        height: 100%;
        border: 1px solid rgba(var(--v-theme-on-surface), 0.12);
        transition: all 0.3s ease;
    }

    .feature-detail-card:hover {
        transform: translateY(-4px);
        box-shadow: 0 8px 24px rgba(0, 0, 0, 0.1);
    }

    .feature-detail-card.highlighted {
        border: 2px solid rgb(var(--v-theme-primary));
    }

    .feature-detail-card h3 {
        font-size: 1.25rem;
        font-weight: 700;
        margin: 16px 0 12px;
    }

    .feature-detail-card p {
        color: rgb(var(--v-theme-on-surface-variant));
        line-height: 1.6;
        margin: 0;
    }

    /* Final CTA Section */
    .final-cta-section {
        padding: 80px 0;
    }

    .cta-card {
        background: linear-gradient(135deg, rgb(var(--v-theme-primary)) 0%, #e85d04 100%);
        color: white;
        text-align: center;
        padding: 60px 40px;
        border-radius: 24px;
        max-width: 1000px;
        margin: 0 auto;
    }

    .sloth-emoji-large {
        font-size: 4rem;
        margin-bottom: 24px;
    }

    .cta-card h2 {
        font-size: clamp(2rem, 4vw, 3rem);
        font-weight: 700;
        margin: 0 0 16px;
        color: white;
    }

    .cta-card p {
        font-size: 1.25rem;
        line-height: 1.6;
        margin: 0 0 40px;
        opacity: 0.95;
    }

    .cta-actions {
        display: flex;
        gap: 16px;
        justify-content: center;
        flex-wrap: wrap;
    }

    @media (max-width: 960px) {
        .home-page {
            padding-top: 60px;
        }

        .hero-section {
            padding: 60px 0;
        }

        .features-section,
        .data-flows-section,
        .flexible-section,
        .final-cta-section {
            padding: 60px 0;
        }
    }
</style>
