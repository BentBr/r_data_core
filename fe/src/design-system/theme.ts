/**
 * Vuetify Theme Configuration
 *
 * Maps design tokens to Vuetify theme structure for both light and dark modes.
 * All colors are sourced from the design system tokens.
 */

import type { ThemeDefinition } from 'vuetify'
import { colorTokens } from './tokens'

/**
 * Light theme configuration
 */
const lightTheme: ThemeDefinition = {
    dark: false,
    colors: {
        // Primary brand color
        primary: colorTokens.light.primary,
        'on-primary': colorTokens.light.primaryForeground,

        // Secondary color
        secondary: colorTokens.light.secondary,
        'on-secondary': colorTokens.light.secondaryForeground,

        // Accent color
        accent: colorTokens.light.accent,
        'on-accent': colorTokens.light.accentForeground,

        // Semantic colors
        success: colorTokens.light.success,
        'on-success': colorTokens.light.successForeground,
        warning: colorTokens.light.warning,
        'on-warning': colorTokens.light.warningForeground,
        error: colorTokens.light.error,
        'on-error': colorTokens.light.errorForeground,
        info: colorTokens.light.info,
        'on-info': colorTokens.light.infoForeground,

        // Surface colors
        surface: colorTokens.light.card,
        'on-surface': colorTokens.light.cardForeground,
        background: colorTokens.light.background,
        'on-background': colorTokens.light.foreground,

        // Extended colors for Vuetify components
        'surface-variant': colorTokens.light.muted,
        'on-surface-variant': colorTokens.light.mutedForeground,

        // Chart colors (available as custom properties)
        'chart-1': colorTokens.light.chart1,
        'chart-2': colorTokens.light.chart2,
        'chart-3': colorTokens.light.chart3,
        'chart-4': colorTokens.light.chart4,
        'chart-5': colorTokens.light.chart5,

        // MDM specific colors
        'mdm-background': colorTokens.light.mdmBackground,
        'mdm-card': colorTokens.light.mdmCard,
        'mdm-border': colorTokens.light.mdmBorder,
        'mdm-muted': colorTokens.light.mdmMuted,
        'mdm-accent': colorTokens.light.mdmAccent,
    },
}

/**
 * Dark theme configuration
 */
const darkTheme: ThemeDefinition = {
    dark: true,
    colors: {
        // Primary brand color
        primary: colorTokens.dark.primary,
        'on-primary': colorTokens.dark.primaryForeground,

        // Secondary color
        secondary: colorTokens.dark.secondary,
        'on-secondary': colorTokens.dark.secondaryForeground,

        // Accent color
        accent: colorTokens.dark.accent,
        'on-accent': colorTokens.dark.accentForeground,

        // Semantic colors
        success: colorTokens.dark.success,
        'on-success': colorTokens.dark.successForeground,
        warning: colorTokens.dark.warning,
        'on-warning': colorTokens.dark.warningForeground,
        error: colorTokens.dark.error,
        'on-error': colorTokens.dark.errorForeground,
        info: colorTokens.dark.info,
        'on-info': colorTokens.dark.infoForeground,

        // Surface colors
        surface: colorTokens.dark.card,
        'on-surface': colorTokens.dark.cardForeground,
        background: colorTokens.dark.background,
        'on-background': colorTokens.dark.foreground,

        // Extended colors for Vuetify components
        'surface-variant': colorTokens.dark.muted,
        'on-surface-variant': colorTokens.dark.mutedForeground,

        // Chart colors (available as custom properties)
        'chart-1': colorTokens.dark.chart1,
        'chart-2': colorTokens.dark.chart2,
        'chart-3': colorTokens.dark.chart3,
        'chart-4': colorTokens.dark.chart4,
        'chart-5': colorTokens.dark.chart5,

        // MDM specific colors
        'mdm-background': colorTokens.dark.mdmBackground,
        'mdm-card': colorTokens.dark.mdmCard,
        'mdm-border': colorTokens.dark.mdmBorder,
        'mdm-muted': colorTokens.dark.mdmMuted,
        'mdm-accent': colorTokens.dark.mdmAccent,
    },
}

/**
 * Vuetify theme configuration
 * This is used in main.ts to create the Vuetify instance
 */
export const vuetifyTheme = {
    defaultTheme: 'light',
    themes: {
        light: lightTheme,
        dark: darkTheme,
    },
} as const
