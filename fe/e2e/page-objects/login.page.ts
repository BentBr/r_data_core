import { type Page, expect } from '@playwright/test'

export class LoginPage {
    constructor(private readonly page: Page) {}

    // Selectors
    private get usernameField() {
        return this.page.getByTestId('login-username').locator('input')
    }

    private get passwordField() {
        return this.page.getByTestId('login-password').locator('input')
    }

    private get submitButton() {
        return this.page.getByTestId('login-submit')
    }

    private get errorAlert() {
        return this.page.getByTestId('login-error')
    }

    // Actions
    async goto(): Promise<void> {
        await this.page.goto('/login')
        await this.page.waitForLoadState('networkidle')
    }

    async login(username: string, password: string): Promise<void> {
        await this.usernameField.fill(username)
        await this.passwordField.fill(password)
        await this.submitButton.click()
    }

    // Assertions
    async expectRedirectToDashboard(): Promise<void> {
        await this.page.waitForURL('**/dashboard', { timeout: 15_000 })
        await expect(this.page).toHaveURL(/\/dashboard/)
    }

    async expectLoginError(): Promise<void> {
        await expect(this.errorAlert).toBeVisible({ timeout: 10_000 })
    }

    async expectOnLoginPage(): Promise<void> {
        await expect(this.page).toHaveURL(/\/login/)
        await expect(this.usernameField).toBeVisible()
    }
}
