/**
 * Vuetify Theme Configuration
 *
 * Maps design tokens to Vuetify theme structure for both light and dark modes.
 * All colors are sourced from the design system tokens.
 */

import type { ThemeDefinition } from 'vuetify'
import { colorTokens } from './tokens'

const lightTheme: ThemeDefinition = {
    dark: false,
    colors: {
        primary: colorTokens.light.primary,
        'on-primary': colorTokens.light.primaryForeground,
        secondary: colorTokens.light.secondary,
        'on-secondary': colorTokens.light.secondaryForeground,
        accent: colorTokens.light.accent,
        'on-accent': colorTokens.light.accentForeground,
        success: colorTokens.light.success,
        'on-success': colorTokens.light.successForeground,
        warning: colorTokens.light.warning,
        'on-warning': colorTokens.light.warningForeground,
        error: colorTokens.light.error,
        'on-error': colorTokens.light.errorForeground,
        info: colorTokens.light.info,
        'on-info': colorTokens.light.infoForeground,
        surface: colorTokens.light.card,
        'on-surface': colorTokens.light.cardForeground,
        background: colorTokens.light.background,
        'on-background': colorTokens.light.foreground,
        'surface-variant': colorTokens.light.muted,
        'on-surface-variant': colorTokens.light.mutedForeground,
        'chart-1': colorTokens.light.chart1,
        'chart-2': colorTokens.light.chart2,
        'chart-3': colorTokens.light.chart3,
        'chart-4': colorTokens.light.chart4,
        'chart-5': colorTokens.light.chart5,
        'mdm-background': colorTokens.light.mdmBackground,
        'mdm-card': colorTokens.light.mdmCard,
        'mdm-border': colorTokens.light.mdmBorder,
        'mdm-muted': colorTokens.light.mdmMuted,
        'mdm-accent': colorTokens.light.mdmAccent,
    },
}

const darkTheme: ThemeDefinition = {
    dark: true,
    colors: {
        primary: colorTokens.dark.primary,
        'on-primary': colorTokens.dark.primaryForeground,
        secondary: colorTokens.dark.secondary,
        'on-secondary': colorTokens.dark.secondaryForeground,
        accent: colorTokens.dark.accent,
        'on-accent': colorTokens.dark.accentForeground,
        success: colorTokens.dark.success,
        'on-success': colorTokens.dark.successForeground,
        warning: colorTokens.dark.warning,
        'on-warning': colorTokens.dark.warningForeground,
        error: colorTokens.dark.error,
        'on-error': colorTokens.dark.errorForeground,
        info: colorTokens.dark.info,
        'on-info': colorTokens.dark.infoForeground,
        surface: colorTokens.dark.card,
        'on-surface': colorTokens.dark.cardForeground,
        background: colorTokens.dark.background,
        'on-background': colorTokens.dark.foreground,
        'surface-variant': colorTokens.dark.muted,
        'on-surface-variant': colorTokens.dark.mutedForeground,
        'chart-1': colorTokens.dark.chart1,
        'chart-2': colorTokens.dark.chart2,
        'chart-3': colorTokens.dark.chart3,
        'chart-4': colorTokens.dark.chart4,
        'chart-5': colorTokens.dark.chart5,
        'mdm-background': colorTokens.dark.mdmBackground,
        'mdm-card': colorTokens.dark.mdmCard,
        'mdm-border': colorTokens.dark.mdmBorder,
        'mdm-muted': colorTokens.dark.mdmMuted,
        'mdm-accent': colorTokens.dark.mdmAccent,
    },
}

export const vuetifyTheme = {
    defaultTheme: 'light',
    themes: {
        light: lightTheme,
        dark: darkTheme,
    },
} as const

export const vuetifyDefaults = {
    VBtn: {
        variant: 'flat',
        color: 'primary',
        size: 'default',
        style: 'border-radius: 8px;',
    },
    VTextField: {
        variant: 'outlined',
        density: 'comfortable',
        color: 'primary',
        style: 'border-radius: 8px;',
    },
    VSelect: {
        variant: 'outlined',
        density: 'comfortable',
        color: 'primary',
        style: 'border-radius: 8px;',
    },
    VTextarea: {
        variant: 'outlined',
        density: 'comfortable',
        color: 'primary',
        style: 'border-radius: 8px;',
    },
    VAutocomplete: {
        variant: 'outlined',
        density: 'comfortable',
        color: 'primary',
        style: 'border-radius: 8px;',
    },
    VCombobox: {
        variant: 'outlined',
        density: 'comfortable',
        color: 'primary',
        style: 'border-radius: 8px;',
    },
    VFileInput: {
        variant: 'outlined',
        density: 'comfortable',
        color: 'primary',
        style: 'border-radius: 8px;',
    },
    VChip: {
        variant: 'flat',
        size: 'default',
        style: 'border-radius: 9999px;',
    },
    VDialog: {
        maxWidth: '600px',
        persistent: false,
    },
    VCard: {
        elevation: 2,
        style: 'border-radius: 12px;',
    },
    VSwitch: {
        color: 'primary',
        inset: true,
    },
    VCheckbox: {
        color: 'primary',
    },
    VRadio: {
        color: 'primary',
    },
} as const
