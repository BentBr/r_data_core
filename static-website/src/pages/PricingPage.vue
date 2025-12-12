<template>
    <div class="pricing-page">
        <!-- Hero Section -->
        <Section class="pricing-hero-section">
            <div class="pricing-hero-container">
                <div class="sloth-emoji-large">🦥</div>
                <div class="pricing-badge">{{ t('pricing.hero.badge') }}</div>
                <h1>{{ t('pricing.hero.title') }}</h1>
                <p class="pricing-subtitle">
                    {{ t('pricing.hero.subtitle') }}
                </p>
                <div class="pricing-callout">
                    {{ t('pricing.hero.callout') }}
                </div>
            </div>
        </Section>

        <!-- What's Included Section -->
        <Section class="whats-included-section">
            <header class="section-heading">
                <h2>{{ t('pricing.whats_included.title') }}</h2>
                <p class="section-subtitle">
                    {{ t('pricing.whats_included.subtitle') }}
                </p>
            </header>
            <v-row>
                <v-col
                    v-for="(feature, index) in get('pricing.whats_included.features')"
                    :key="index"
                    cols="12"
                    sm="6"
                    md="4"
                    lg="3"
                >
                    <div class="feature-check">
                        <SmartIcon
                            icon="check"
                            color="primary"
                            size="sm"
                        />
                        <span>{{ feature }}</span>
                    </div>
                </v-col>
            </v-row>
        </Section>

        <!-- Flexibility Callout -->
        <Section class="flex-callout-section">
            <v-card
                class="flex-callout-card"
                elevation="0"
            >
                <v-card-text>
                    <div class="flex-icon">
                        <SmartIcon
                            icon="zap"
                            color="primary"
                            size="xl"
                        />
                    </div>
                    <h3>{{ t('pricing.flexibility.title') }}</h3>
                    <p>
                        {{ t('pricing.flexibility.desc') }}
                    </p>
                </v-card-text>
            </v-card>
        </Section>

        <!-- Cloud Coming Soon -->
        <Section class="cloud-banner-section">
            <v-card
                class="cloud-banner"
                elevation="2"
            >
                <v-card-text>
                    <div class="cloud-content">
                        <div class="cloud-icon">☁️</div>
                        <div class="cloud-text">
                            <h3>{{ t('pricing.cloud.title') }}</h3>
                            <p>
                                {{ t('pricing.cloud.desc') }}
                            </p>
                        </div>
                        <v-btn
                            variant="outlined"
                            rounded
                        >
                            <SmartIcon
                                icon="bell"
                                size="sm"
                            />
                            {{ t('pricing.cloud.notify') }}
                        </v-btn>
                    </div>
                </v-card-text>
            </v-card>
        </Section>

        <!-- Free Licenses Section -->
        <Section class="free-licenses-section">
            <header class="section-heading">
                <h2>{{ t('pricing.free_licenses.title') }}</h2>
                <p class="section-subtitle">
                    {{ t('pricing.free_licenses.subtitle') }}
                </p>
            </header>
            <v-row>
                <v-col
                    cols="12"
                    lg="6"
                >
                    <v-card
                        class="pricing-tier-card"
                        elevation="2"
                    >
                        <div class="free-badge">
                            {{ t('pricing.free_licenses.community.badge') }}
                        </div>
                        <v-card-text>
                            <h3>{{ t('pricing.free_licenses.community.title') }}</h3>
                            <p class="tier-desc">
                                {{ t('pricing.free_licenses.community.desc') }}
                            </p>
                            <div class="price">
                                <span class="price-amount">{{
                                    t('pricing.free_licenses.community.price')
                                }}</span>
                                <span class="price-period">{{
                                    t('pricing.free_licenses.community.period')
                                }}</span>
                            </div>
                            <ul class="tier-features">
                                <li
                                    v-for="(feature, index) in get(
                                        'pricing.free_licenses.community.features'
                                    )"
                                    :key="index"
                                >
                                    <SmartIcon
                                        icon="check"
                                        color="primary"
                                        size="sm"
                                    />
                                    {{ feature }}
                                </li>
                            </ul>
                            <p class="tier-footnote">
                                {{ t('pricing.free_licenses.community.footnote') }}
                            </p>
                            <v-btn
                                block
                                size="large"
                                rounded
                                variant="outlined"
                            >
                                {{ t('pricing.free_licenses.community.button') }}
                            </v-btn>
                        </v-card-text>
                    </v-card>
                </v-col>
                <v-col
                    cols="12"
                    lg="6"
                >
                    <v-card
                        class="pricing-tier-card"
                        elevation="2"
                    >
                        <div class="free-badge">
                            {{ t('pricing.free_licenses.education.badge') }}
                        </div>
                        <v-card-text>
                            <h3>{{ t('pricing.free_licenses.education.title') }}</h3>
                            <p class="tier-desc">
                                {{ t('pricing.free_licenses.education.desc') }}
                            </p>
                            <div class="price">
                                <span class="price-amount">{{
                                    t('pricing.free_licenses.education.price')
                                }}</span>
                                <span class="price-period">{{
                                    t('pricing.free_licenses.education.period')
                                }}</span>
                            </div>
                            <ul class="tier-features">
                                <li
                                    v-for="(feature, index) in get(
                                        'pricing.free_licenses.education.features'
                                    )"
                                    :key="index"
                                >
                                    <SmartIcon
                                        icon="check"
                                        color="primary"
                                        size="sm"
                                    />
                                    {{ feature }}
                                </li>
                            </ul>
                            <p class="tier-footnote">
                                {{ t('pricing.free_licenses.education.footnote') }}
                            </p>
                            <v-btn
                                block
                                size="large"
                                rounded
                                variant="outlined"
                            >
                                {{ t('pricing.free_licenses.education.button') }}
                            </v-btn>
                        </v-card-text>
                    </v-card>
                </v-col>
            </v-row>
        </Section>

        <!-- Company Licenses Section -->
        <Section class="company-licenses-section">
            <header class="section-heading">
                <h2>{{ t('pricing.company_licenses.title') }}</h2>
                <p class="section-subtitle">
                    {{ t('pricing.company_licenses.subtitle') }}
                </p>
                <div class="feature-equality">
                    {{ t('pricing.company_licenses.equality') }}
                </div>
            </header>
            <v-row>
                <v-col
                    cols="12"
                    lg="4"
                >
                    <v-card
                        class="pricing-tier-card"
                        elevation="0"
                    >
                        <v-card-text>
                            <h3>Company I</h3>
                            <p class="tier-desc">5M€ or 10+ Employees</p>
                            <p class="tier-subdesc">Fair pricing for growing companies</p>
                            <div class="price">
                                <span class="price-amount">400€</span>
                                <span class="price-period">/month</span>
                            </div>
                            <div class="price-annual">
                                <span class="annual-amount">4,500€</span>
                                <span class="annual-period">/year</span>
                                <span class="annual-discount">6.25% off</span>
                            </div>
                            <ul class="tier-features">
                                <li>
                                    <SmartIcon
                                        icon="check"
                                        color="primary"
                                        size="sm"
                                    />
                                    All 18 features included
                                </li>
                                <li>
                                    <SmartIcon
                                        icon="check"
                                        color="primary"
                                        size="sm"
                                    />
                                    Email & chat support
                                </li>
                                <li>
                                    <SmartIcon
                                        icon="check"
                                        color="primary"
                                        size="sm"
                                    />
                                    Regular updates & patches
                                </li>
                                <li>
                                    <SmartIcon
                                        icon="check"
                                        color="primary"
                                        size="sm"
                                    />
                                    Commercial license
                                </li>
                            </ul>
                            <v-btn
                                block
                                size="large"
                                rounded
                                variant="outlined"
                            >
                                Get License
                            </v-btn>
                        </v-card-text>
                    </v-card>
                </v-col>
                <v-col
                    cols="12"
                    lg="4"
                >
                    <v-card
                        class="pricing-tier-card popular"
                        elevation="4"
                    >
                        <div class="popular-badge">⭐ Most Popular</div>
                        <v-card-text>
                            <h3>Company II</h3>
                            <p class="tier-desc">15M€ or 40+ Employees</p>
                            <p class="tier-subdesc">Most common for mid-size organizations</p>
                            <div class="price">
                                <span class="price-amount">900€</span>
                                <span class="price-period">/month</span>
                            </div>
                            <div class="price-annual">
                                <span class="annual-amount">10,000€</span>
                                <span class="annual-period">/year</span>
                                <span class="annual-discount">8.3% off</span>
                            </div>
                            <ul class="tier-features">
                                <li>
                                    <SmartIcon
                                        icon="check"
                                        color="primary"
                                        size="sm"
                                    />
                                    All 18 features included
                                </li>
                                <li>
                                    <SmartIcon
                                        icon="check"
                                        color="primary"
                                        size="sm"
                                    />
                                    Email & chat support
                                </li>
                                <li>
                                    <SmartIcon
                                        icon="check"
                                        color="primary"
                                        size="sm"
                                    />
                                    Regular updates & patches
                                </li>
                                <li>
                                    <SmartIcon
                                        icon="check"
                                        color="primary"
                                        size="sm"
                                    />
                                    Commercial license
                                </li>
                            </ul>
                            <v-btn
                                block
                                size="large"
                                rounded
                                color="primary"
                            >
                                Get License
                            </v-btn>
                        </v-card-text>
                    </v-card>
                </v-col>
                <v-col
                    cols="12"
                    lg="4"
                >
                    <v-card
                        class="pricing-tier-card"
                        elevation="0"
                    >
                        <v-card-text>
                            <h3>Company III</h3>
                            <p class="tier-desc">150M€ or 100+ Employees</p>
                            <p class="tier-subdesc">Enterprise-scale organizations</p>
                            <div class="price">
                                <span class="price-amount">3,000€</span>
                                <span class="price-period">/month</span>
                            </div>
                            <div class="price-annual">
                                <span class="annual-amount">30,000€</span>
                                <span class="annual-period">/year</span>
                                <span class="annual-discount">17% off</span>
                            </div>
                            <ul class="tier-features">
                                <li>
                                    <SmartIcon
                                        icon="check"
                                        color="primary"
                                        size="sm"
                                    />
                                    All 18 features included
                                </li>
                                <li>
                                    <SmartIcon
                                        icon="check"
                                        color="primary"
                                        size="sm"
                                    />
                                    Email & chat support
                                </li>
                                <li>
                                    <SmartIcon
                                        icon="check"
                                        color="primary"
                                        size="sm"
                                    />
                                    Regular updates & patches
                                </li>
                                <li>
                                    <SmartIcon
                                        icon="check"
                                        color="primary"
                                        size="sm"
                                    />
                                    Commercial license
                                </li>
                            </ul>
                            <v-btn
                                block
                                size="large"
                                rounded
                                variant="outlined"
                            >
                                Get License
                            </v-btn>
                        </v-card-text>
                    </v-card>
                </v-col>
            </v-row>
        </Section>

        <!-- Society Licenses Section -->
        <Section class="society-licenses-section">
            <header class="section-heading">
                <h2>{{ t('pricing.society_licenses.title') }}</h2>
                <p class="section-subtitle">
                    {{ t('pricing.society_licenses.subtitle') }}
                </p>
                <div class="feature-equality">
                    {{ t('pricing.society_licenses.equality') }}
                </div>
            </header>
            <v-row>
                <v-col
                    cols="12"
                    lg="4"
                >
                    <v-card
                        class="pricing-tier-card"
                        elevation="0"
                    >
                        <v-card-text>
                            <h3>Society I</h3>
                            <p class="tier-desc">
                                10+ Employees (NGOs, Private schools, Private universities)
                            </p>
                            <div class="price">
                                <span class="price-amount">200€</span>
                                <span class="price-period">/month</span>
                            </div>
                            <div class="price-annual">
                                <span class="annual-amount">2,250€</span>
                                <span class="annual-period">/year</span>
                                <span class="annual-discount">6.7% off</span>
                            </div>
                            <ul class="tier-features">
                                <li>
                                    <SmartIcon
                                        icon="check"
                                        color="primary"
                                        size="sm"
                                    />
                                    All features included
                                </li>
                                <li>
                                    <SmartIcon
                                        icon="check"
                                        color="primary"
                                        size="sm"
                                    />
                                    Email support
                                </li>
                            </ul>
                            <v-btn
                                block
                                size="large"
                                rounded
                                variant="outlined"
                            >
                                Get License
                            </v-btn>
                        </v-card-text>
                    </v-card>
                </v-col>
                <v-col
                    cols="12"
                    lg="4"
                >
                    <v-card
                        class="pricing-tier-card"
                        elevation="0"
                    >
                        <v-card-text>
                            <h3>Society II</h3>
                            <p class="tier-desc">40+ Employees</p>
                            <div class="price">
                                <span class="price-amount">450€</span>
                                <span class="price-period">/month</span>
                            </div>
                            <div class="price-annual">
                                <span class="annual-amount">5,000€</span>
                                <span class="annual-period">/year</span>
                                <span class="annual-discount">7.4% off</span>
                            </div>
                            <ul class="tier-features">
                                <li>
                                    <SmartIcon
                                        icon="check"
                                        color="primary"
                                        size="sm"
                                    />
                                    All features included
                                </li>
                                <li>
                                    <SmartIcon
                                        icon="check"
                                        color="primary"
                                        size="sm"
                                    />
                                    Email support
                                </li>
                            </ul>
                            <v-btn
                                block
                                size="large"
                                rounded
                                variant="outlined"
                            >
                                Get License
                            </v-btn>
                        </v-card-text>
                    </v-card>
                </v-col>
                <v-col
                    cols="12"
                    lg="4"
                >
                    <v-card
                        class="pricing-tier-card"
                        elevation="0"
                    >
                        <v-card-text>
                            <h3>Society III</h3>
                            <p class="tier-desc">100+ Employees</p>
                            <div class="price">
                                <span class="price-amount">1,500€</span>
                                <span class="price-period">/month</span>
                            </div>
                            <div class="price-annual">
                                <span class="annual-amount">15,000€</span>
                                <span class="annual-period">/year</span>
                                <span class="annual-discount">1.7% off</span>
                            </div>
                            <ul class="tier-features">
                                <li>
                                    <SmartIcon
                                        icon="check"
                                        color="primary"
                                        size="sm"
                                    />
                                    All features included
                                </li>
                                <li>
                                    <SmartIcon
                                        icon="check"
                                        color="primary"
                                        size="sm"
                                    />
                                    Email support
                                </li>
                            </ul>
                            <v-btn
                                block
                                size="large"
                                rounded
                                variant="outlined"
                            >
                                Get License
                            </v-btn>
                        </v-card-text>
                    </v-card>
                </v-col>
            </v-row>
        </Section>

        <!-- Corporate Edition CTA -->
        <Section class="corporate-cta-section">
            <v-card
                class="corporate-card"
                elevation="8"
            >
                <v-card-text>
                    <div class="corporate-icon">✨</div>
                    <h2>{{ t('pricing.corporate.title') }}</h2>
                    <p>
                        {{ t('pricing.corporate.desc') }}
                    </p>
                    <div class="corporate-actions">
                        <v-btn
                            color="white"
                            size="x-large"
                            rounded
                            elevation="0"
                            @click="openContact"
                        >
                            {{ t('pricing.corporate.schedule_call') }}
                        </v-btn>
                        <v-btn
                            variant="outlined"
                            color="white"
                            size="x-large"
                            rounded
                            @click="openContact"
                        >
                            {{ t('pricing.corporate.get_quote') }}
                        </v-btn>
                    </div>
                </v-card-text>
            </v-card>
        </Section>

        <!-- FAQ Section -->
        <Section class="faq-section">
            <header class="section-heading">
                <h2>{{ t('pricing.faq.title') }}</h2>
            </header>
            <div class="faq-container">
                <v-card
                    v-for="(faq, index) in get('pricing.faq.items') || []"
                    :key="index"
                    class="faq-card"
                    elevation="0"
                >
                    <v-card-text>
                        <h3>{{ (faq as any).question }}</h3>
                        <p>{{ (faq as any).answer }}</p>
                    </v-card-text>
                </v-card>
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

    const { t, get } = useTranslations()
    const showDemoDialog = ref(false)

    const openContact = () => {
        const email = t('contact.email')
        window.open(`mailto:${email}`, '_blank', 'noopener,noreferrer')
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
        title: t('seo.pricing.title'),
        description: t('seo.pricing.description'),
    })
</script>

<style scoped>
    .pricing-page {
        padding-top: 80px;
    }

    /* Hero Section */
    .pricing-hero-section {
        padding: 80px 0;
        text-align: center;
        background: linear-gradient(
            180deg,
            rgba(var(--v-theme-surface), 1) 0%,
            rgba(var(--v-theme-surface-variant), 0.3) 100%
        );
    }

    .pricing-hero-container {
        max-width: 900px;
        margin: 0 auto;
    }

    .sloth-emoji-large {
        font-size: 4rem;
        margin-bottom: 16px;
    }

    .pricing-badge {
        display: inline-block;
        background: rgb(var(--v-theme-primary));
        color: white;
        padding: 8px 20px;
        border-radius: 24px;
        font-size: 0.875rem;
        font-weight: 600;
        margin-bottom: 24px;
    }

    .pricing-hero-section h1 {
        font-size: clamp(2.5rem, 5vw, 4rem);
        font-weight: 800;
        line-height: 1.1;
        margin: 0 0 24px;
    }

    .pricing-subtitle {
        font-size: 1.25rem;
        line-height: 1.6;
        color: rgb(var(--v-theme-on-surface-variant));
        margin: 0 0 32px;
    }

    .pricing-callout {
        display: inline-block;
        background: rgba(var(--v-theme-primary), 0.1);
        color: rgb(var(--v-theme-on-surface));
        padding: 12px 24px;
        border-radius: 24px;
        font-size: 1rem;
        font-weight: 600;
        border: 2px solid rgba(var(--v-theme-primary), 0.3);
    }

    /* What's Included Section */
    .whats-included-section {
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
    }

    .section-subtitle {
        font-size: 1.125rem;
        color: rgb(var(--v-theme-on-surface-variant));
        max-width: 700px;
        margin: 0 auto 24px;
        line-height: 1.6;
    }

    .feature-equality {
        display: inline-block;
        background: rgba(var(--v-theme-primary), 0.1);
        padding: 10px 20px;
        border-radius: 20px;
        font-weight: 600;
        color: rgb(var(--v-theme-primary));
    }

    .feature-check {
        display: flex;
        align-items: center;
        gap: 12px;
        padding: 8px 0;
    }

    /* Flex Callout Section */
    .flex-callout-section {
        padding: 40px 0;
    }

    .flex-callout-card {
        border: 2px solid rgba(var(--v-theme-primary), 0.3);
        border-radius: 16px;
        max-width: 900px;
        margin: 0 auto;
        padding: 24px;
    }

    .flex-icon {
        text-align: center;
        margin-bottom: 16px;
    }

    .flex-callout-card h3 {
        font-size: 1.75rem;
        font-weight: 700;
        text-align: center;
        margin-bottom: 16px;
    }

    .flex-callout-card p {
        font-size: 1.125rem;
        line-height: 1.7;
        color: rgb(var(--v-theme-on-surface-variant));
        margin: 0;
        text-align: center;
    }

    /* Cloud Banner Section */
    .cloud-banner-section {
        padding: 40px 0;
    }

    .cloud-banner {
        border: 2px solid rgba(var(--v-theme-primary), 0.3);
        border-radius: 16px;
        max-width: 900px;
        margin: 0 auto;
    }

    .cloud-content {
        display: flex;
        align-items: center;
        gap: 24px;
        flex-wrap: wrap;
    }

    .cloud-icon {
        font-size: 3rem;
    }

    .cloud-text {
        flex: 1;
        min-width: 300px;
    }

    .cloud-text h3 {
        font-size: 1.5rem;
        font-weight: 700;
        margin: 0 0 8px;
    }

    .cloud-text p {
        color: rgb(var(--v-theme-on-surface-variant));
        margin: 0;
    }

    /* License Sections */
    .free-licenses-section,
    .company-licenses-section,
    .society-licenses-section {
        padding: 80px 0;
    }

    .company-licenses-section {
        background: rgba(var(--v-theme-surface-variant), 0.2);
    }

    /* Ensure v-col allows badge overflow */
    .free-licenses-section :deep(.v-col),
    .company-licenses-section :deep(.v-col) {
        overflow: visible;
    }

    .free-licenses-section :deep(.v-row),
    .company-licenses-section :deep(.v-row) {
        overflow: visible;
    }

    .pricing-tier-card {
        height: 100%;
        border: 1px solid rgba(var(--v-theme-on-surface), 0.12);
        border-radius: 16px;
        padding: 16px;
        padding-top: 32px; /* Extra space for badge */
        position: relative;
        transition: transform 0.3s ease;
        margin-top: 20px; /* Space above card for badge */
        overflow: visible; /* Allow badge to overflow */
    }

    .pricing-tier-card:hover {
        transform: translateY(-8px);
    }

    .pricing-tier-card.popular {
        border: 2px solid rgb(var(--v-theme-primary));
    }

    .free-badge {
        position: absolute;
        top: -14px;
        right: 16px;
        background: rgb(var(--v-theme-primary));
        color: white;
        padding: 6px 16px;
        border-radius: 20px;
        font-size: 0.875rem;
        font-weight: 600;
        box-shadow: 0 2px 8px rgba(var(--v-theme-primary), 0.4);
        white-space: nowrap;
        z-index: 10;
    }

    .popular-badge {
        position: absolute;
        top: -14px;
        left: 50%;
        transform: translateX(-50%);
        background: rgb(var(--v-theme-primary));
        color: white;
        padding: 6px 16px;
        border-radius: 20px;
        font-size: 0.875rem;
        font-weight: 600;
        box-shadow: 0 2px 8px rgba(var(--v-theme-primary), 0.4);
        white-space: nowrap;
        z-index: 10;
    }

    .pricing-tier-card h3 {
        font-size: 1.75rem;
        font-weight: 700;
        margin-bottom: 8px;
    }

    .tier-desc {
        color: rgb(var(--v-theme-on-surface-variant));
        margin-bottom: 4px;
        font-size: 0.95rem;
    }

    .tier-subdesc {
        color: rgb(var(--v-theme-on-surface-variant));
        margin-bottom: 24px;
        font-size: 0.9rem;
        font-style: italic;
    }

    .price {
        margin-bottom: 8px;
    }

    .price-amount {
        font-size: 3rem;
        font-weight: 800;
        color: rgb(var(--v-theme-on-surface));
    }

    .price-period {
        font-size: 1.25rem;
        color: rgb(var(--v-theme-on-surface-variant));
    }

    .price-annual {
        margin-bottom: 24px;
        font-size: 1.125rem;
    }

    .annual-amount {
        font-weight: 700;
        color: rgb(var(--v-theme-on-surface));
    }

    .annual-period {
        color: rgb(var(--v-theme-on-surface-variant));
    }

    .annual-discount {
        margin-left: 8px;
        color: rgb(var(--v-theme-primary));
        font-weight: 700;
        background: rgba(var(--v-theme-primary), 0.1);
        padding: 2px 8px;
        border-radius: 8px;
        font-size: 0.9rem;
    }

    .tier-features {
        list-style: none;
        padding: 0;
        margin: 0 0 16px;
    }

    .tier-features li {
        display: flex;
        align-items: center;
        gap: 12px;
        padding: 8px 0;
    }

    .tier-footnote {
        color: rgb(var(--v-theme-on-surface-variant));
        font-size: 0.875rem;
        margin-bottom: 24px;
        font-style: italic;
    }

    /* Corporate Section */
    .corporate-cta-section {
        padding: 80px 0;
    }

    .corporate-card {
        background: linear-gradient(135deg, rgb(var(--v-theme-primary)) 0%, #e85d04 100%);
        color: white;
        text-align: center;
        padding: 60px 40px;
        border-radius: 24px;
        max-width: 900px;
        margin: 0 auto;
    }

    .corporate-icon {
        font-size: 4rem;
        margin-bottom: 24px;
    }

    .corporate-card h2 {
        font-size: clamp(2rem, 4vw, 3rem);
        font-weight: 700;
        margin: 0 0 24px;
        color: white;
    }

    .corporate-card p {
        font-size: 1.375rem;
        line-height: 1.7;
        margin: 0 0 40px;
        opacity: 0.95;
    }

    .corporate-actions {
        display: flex;
        gap: 16px;
        justify-content: center;
        flex-wrap: wrap;
    }

    /* FAQ Section */
    .faq-section {
        padding: 80px 0;
        background: rgba(var(--v-theme-surface-variant), 0.1);
    }

    .faq-container {
        max-width: 900px;
        margin: 0 auto;
        display: grid;
        gap: 24px;
    }

    .faq-card {
        border: 1px solid rgba(var(--v-theme-on-surface), 0.12);
        border-radius: 12px;
        padding: 16px;
    }

    .faq-card h3 {
        font-size: 1.25rem;
        font-weight: 700;
        margin-bottom: 12px;
    }

    .faq-card p {
        color: rgb(var(--v-theme-on-surface-variant));
        line-height: 1.6;
        margin: 0;
    }

    @media (max-width: 960px) {
        .pricing-page {
            padding-top: 60px;
        }

        .pricing-hero-section,
        .whats-included-section,
        .free-licenses-section,
        .company-licenses-section,
        .society-licenses-section,
        .corporate-cta-section,
        .faq-section {
            padding: 60px 0;
        }

        .cloud-content {
            flex-direction: column;
            text-align: center;
        }
    }
</style>
