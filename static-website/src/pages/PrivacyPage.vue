<template>
    <div class="privacy-page">
        <Section class="privacy-section">
            <div class="privacy-container">
                <h1>{{ t('privacy.title') }}</h1>

                <div class="privacy-content">
                    <section>
                        <h2>{{ t('privacy.intro.title') }}</h2>
                        <p>{{ t('privacy.intro.text') }}</p>
                    </section>

                    <!-- <section>
                        <h2>{{ t('privacy.controller.title') }}</h2>
                        <p>
                            Slothlike GmbH<br />
                            Musterstraße 123<br />
                            12345 Berlin<br />
                            Germany<br /><br />
                            {{ t('imprint.contact.email') }}:
                            <a :href="`mailto:${t('contact.email')}`">{{ t('contact.email') }}</a>
                        </p>
                    </section> -->

                    <section>
                        <h2>{{ t('privacy.data_collection.title') }}</h2>
                        <p>{{ t('privacy.data_collection.text') }}</p>
                    </section>

                    <section>
                        <h2>{{ t('privacy.cookies.title') }}</h2>
                        <p>{{ t('privacy.cookies.text') }}</p>
                    </section>

                    <section>
                        <h2>{{ t('privacy.analytics.title') }}</h2>
                        <p>{{ t('privacy.analytics.text') }}</p>
                    </section>

                    <section>
                        <h2>{{ t('privacy.your_rights.title') }}</h2>
                        <p>{{ t('privacy.your_rights.text') }}</p>
                        <ul>
                            <li>{{ t('privacy.your_rights.items.0') }}</li>
                            <li>{{ t('privacy.your_rights.items.1') }}</li>
                            <li>{{ t('privacy.your_rights.items.2') }}</li>
                            <li>{{ t('privacy.your_rights.items.3') }}</li>
                            <li>{{ t('privacy.your_rights.items.4') }}</li>
                        </ul>
                    </section>

                    <section>
                        <h2>{{ t('privacy.data_security.title') }}</h2>
                        <p>{{ t('privacy.data_security.text') }}</p>
                    </section>

                    <section>
                        <h2>{{ t('privacy.changes.title') }}</h2>
                        <p>{{ t('privacy.changes.text') }}</p>
                    </section>

                    <section>
                        <h2>{{ t('privacy.contact.title') }}</h2>
                        <p>{{ t('privacy.contact.text') }}</p>
                    </section>
                </div>
            </div>
        </Section>
    </div>
</template>

<script setup lang="ts">
    import Section from '@/components/Section.vue'
    import { useTranslations } from '@/composables/useTranslations'
    import { useSEO } from '@/composables/useSEO'

    const { t } = useTranslations()

    useSEO({
        title: () => t('privacy.seo.title'),
        description: () => t('privacy.seo.description'),
    })
</script>

<style scoped>
    .privacy-page {
        padding-top: 80px;
        min-height: 100vh;
    }

    .privacy-section {
        padding: 80px 0;
    }

    .privacy-container {
        max-width: 800px;
        margin: 0 auto;
    }

    h1 {
        font-size: clamp(2rem, 4vw, 3rem);
        font-weight: 700;
        margin: 0 0 40px;
        color: rgb(var(--v-theme-on-surface));
    }

    .privacy-content {
        display: grid;
        gap: 32px;
    }

    section {
        padding: 24px 0;
        border-bottom: 1px solid rgba(var(--v-theme-on-surface), 0.1);
    }

    section:last-child {
        border-bottom: none;
    }

    h2 {
        font-size: 1.5rem;
        font-weight: 700;
        margin: 0 0 16px;
        color: rgb(var(--v-theme-on-surface));
    }

    p {
        color: rgb(var(--v-theme-on-surface-variant));
        line-height: 1.7;
        margin: 0 0 16px;
    }

    ul {
        color: rgb(var(--v-theme-on-surface-variant));
        line-height: 1.7;
        margin: 16px 0;
        padding-left: 24px;
    }

    li {
        margin: 8px 0;
    }

    a {
        color: rgb(var(--v-theme-primary));
        text-decoration: none;
    }

    a:hover {
        text-decoration: underline;
    }

    @media (max-width: 960px) {
        .privacy-page {
            padding-top: 60px;
        }

        .privacy-section {
            padding: 60px 0;
        }
    }
</style>
