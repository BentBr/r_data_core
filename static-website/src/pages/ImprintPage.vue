<template>
    <div class="imprint-page">
        <Section class="imprint-section">
            <div class="imprint-container">
                <h1>{{ t('imprint.title') }}</h1>

                <div class="imprint-content">
                    <!-- <section>
                        <h2>{{ t('imprint.company.title') }}</h2>
                        <p v-html="t('imprint.company.address')"></p>
                    </section> -->

                    <section>
                        <h2>{{ t('imprint.contact.title') }}</h2>
                        <p>
                            {{ t('imprint.contact.email') }}:
                            <a :href="`mailto:${t('contact.email')}`">{{ t('contact.email') }}</a
                            ><br />
                            {{ t('imprint.contact.phone') }}:
                            {{ t('imprint.contact.phone_number') }}
                        </p>
                    </section>

                    <!-- <section>
                        <h2>{{ t('imprint.legal.title') }}</h2>
                        <p>
                            {{ t('imprint.legal.registration') }}:
                            {{ t('imprint.legal.registration_number') }}<br />
                            {{ t('imprint.legal.court') }}: {{ t('imprint.legal.court_name')
                            }}<br />
                            {{ t('imprint.legal.vat') }}: {{ t('imprint.legal.vat_number') }}
                        </p>
                    </section> -->

                    <section>
                        <h2>{{ t('imprint.responsible.title') }}</h2>
                        <p>{{ t('imprint.responsible.text') }}</p>
                    </section>

                    <section>
                        <h2>{{ t('imprint.disclaimer.title') }}</h2>
                        <p>{{ t('imprint.disclaimer.content') }}</p>
                    </section>

                    <section>
                        <h2>{{ t('imprint.liability.title') }}</h2>
                        <p>{{ t('imprint.liability.content') }}</p>
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
        title: () => t('imprint.seo.title'),
        description: () => t('imprint.seo.description'),
    })
</script>

<style scoped>
    .imprint-page {
        padding-top: 80px;
        min-height: 100vh;
    }

    .imprint-section {
        padding: 80px 0;
    }

    .imprint-container {
        max-width: 800px;
        margin: 0 auto;
    }

    h1 {
        font-size: clamp(2rem, 4vw, 3rem);
        font-weight: 700;
        margin: 0 0 40px;
        color: rgb(var(--v-theme-on-surface));
    }

    .imprint-content {
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
        margin: 0;
    }

    a {
        color: rgb(var(--v-theme-primary));
        text-decoration: none;
    }

    a:hover {
        text-decoration: underline;
    }

    @media (max-width: 960px) {
        .imprint-page {
            padding-top: 60px;
        }

        .imprint-section {
            padding: 60px 0;
        }
    }
</style>
