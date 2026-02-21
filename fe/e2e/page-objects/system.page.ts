import { type Page, expect } from '@playwright/test'

export class SystemPage {
    constructor(private readonly page: Page) {}

    // Selectors
    private get licenseCard() {
        return this.page.getByTestId('system-license-card')
    }

    private get versioningCard() {
        return this.page.getByTestId('system-versioning-card')
    }

    private get runLogsCard() {
        return this.page.getByTestId('system-run-logs-card')
    }

    private get versioningSaveBtn() {
        return this.page.getByTestId('system-versioning-save')
    }

    private get runLogsSaveBtn() {
        return this.page.getByTestId('system-run-logs-save')
    }

    // Actions
    async goto(): Promise<void> {
        await this.page.goto('/system')
        await this.page.waitForLoadState('networkidle')
    }

    async updateVersioning(maxVersions: string): Promise<void> {
        const maxVersionsInput = this.versioningCard.getByLabel(/max.versions/i)
        await maxVersionsInput.clear()
        await maxVersionsInput.fill(maxVersions)
    }

    async saveVersioning(): Promise<void> {
        await this.versioningSaveBtn.click()
    }

    async updateRunLogs(maxRuns: string): Promise<void> {
        const maxRunsInput = this.runLogsCard.getByLabel(/max.runs/i)
        await maxRunsInput.clear()
        await maxRunsInput.fill(maxRuns)
    }

    async saveRunLogs(): Promise<void> {
        await this.runLogsSaveBtn.click()
    }

    // Assertions
    async expectLicenseLoaded(): Promise<void> {
        await expect(this.licenseCard).toBeVisible()
        // Wait for skeleton to disappear (license data loaded)
        await expect(this.licenseCard.locator('.v-skeleton-loader')).not.toBeVisible({
            timeout: 10_000,
        })
    }

    async expectVersioningSettings(): Promise<void> {
        await expect(this.versioningCard).toBeVisible()
        await expect(this.versioningSaveBtn).toBeVisible()
    }

    async expectRunLogsSettings(): Promise<void> {
        await expect(this.runLogsCard).toBeVisible()
        await expect(this.runLogsSaveBtn).toBeVisible()
    }
}
