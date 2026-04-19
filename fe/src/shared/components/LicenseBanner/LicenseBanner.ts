import { computed, defineComponent } from 'vue'
import { useLicenseStore } from '@/stores/license'
import { useTranslations } from '@/shared/composables/useTranslations'
import DismissableBanner from '@/shared/components/DismissableBanner/index.vue'

export default defineComponent({
    name: 'LicenseBanner',
    components: {
        DismissableBanner,
    },
    setup() {
        const licenseStore = useLicenseStore()
        const { t } = useTranslations()

        const bannerMessage = computed(() => {
            const state = licenseStore.licenseStatus?.state
            if (!state || state === 'none') return t('license.banner.no_license')
            if (state === 'invalid') return t('license.banner.invalid_license')
            if (state === 'error') return t('license.banner.error_license')
            return t('license.banner.no_license')
        })

        return { licenseStore, t, bannerMessage, handleDismiss: () => { licenseStore.dismissLicenseBanner() } }
    },
})
