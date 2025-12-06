/**
 * Design Tokens
 *
 * Complete design system tokens extracted from Figma design system.
 * All colors, spacing, typography, and other design values are defined here.
 */

/**
 * Color tokens for light and dark themes
 * Extracted from Figma design system: https://www.figma.com/make/TfyVgKu2VFjIRq4frfI3CM/RDataCore
 */
export const colorTokens = {
    light: {
        // Brand Colors
        primary: '#ff6b00',
        primaryForeground: '#ffffff',

        // Background Colors
        background: '#ffffff',
        foreground: '#252525', // oklch(0.145 0 0) converted to hex
        card: '#ffffff',
        cardForeground: '#252525', // oklch(0.145 0 0) converted to hex

        // Accent Colors
        secondary: '#fef3ec',
        secondaryForeground: '#ff6b00',
        accent: '#fff4ed',
        accentForeground: '#ff6b00',

        // UI Element Colors
        muted: '#f7f7f7',
        mutedForeground: '#6b7280',
        border: 'rgba(0, 0, 0, 0.08)',
        input: 'transparent',
        ring: '#ff6b00',

        // Informational Colors
        info: '#0284c7',
        infoForeground: '#ffffff',
        success: '#16a34a',
        successForeground: '#ffffff',
        warning: '#ea580c',
        warningForeground: '#ffffff',
        error: '#dc2626',
        errorForeground: '#ffffff',
        destructive: '#d4183d',
        destructiveForeground: '#ffffff',

        // Chart Colors
        chart1: '#ff6b00',
        chart2: '#3b82f6',
        chart3: '#8b5cf6',
        chart4: '#10b981',
        chart5: '#f59e0b',

        // MDM Tool Specific
        mdmBackground: '#fafafa',
        mdmCard: '#ffffff',
        mdmBorder: '#e5e5e5',
        mdmMuted: '#f5f5f5',
        mdmAccent: '#fef3ec',
    },
    dark: {
        // Brand Colors
        primary: '#ff8533',
        primaryForeground: '#000000',

        // Background Colors
        background: '#0a0a0a',
        foreground: '#fafafa', // oklch(0.985 0 0) converted to hex
        card: '#171717',
        cardForeground: '#fafafa', // oklch(0.985 0 0) converted to hex

        // Accent Colors
        secondary: '#454545', // oklch(0.269 0 0) converted to hex
        secondaryForeground: '#fafafa', // oklch(0.985 0 0) converted to hex
        accent: '#2e1a0d',
        accentForeground: '#ff8533',

        // UI Element Colors
        muted: '#262626',
        mutedForeground: '#a3a3a3',
        border: '#262626',
        input: '#262626',
        ring: '#ff8533',

        // Informational Colors
        info: '#0ea5e9',
        infoForeground: '#ffffff',
        success: '#22c55e',
        successForeground: '#ffffff',
        warning: '#f97316',
        warningForeground: '#ffffff',
        error: '#ef4444',
        errorForeground: '#ffffff',
        destructive: '#d4183d', // oklch(0.396 0.141 25.723) - using same as light for consistency
        destructiveForeground: '#ff6b6b', // oklch(0.637 0.237 25.331) converted to hex

        // Chart Colors
        chart1: '#ff8533',
        chart2: '#0ea5e9',
        chart3: '#a78bfa',
        chart4: '#34d399',
        chart5: '#fbbf24',

        // MDM Tool Specific
        mdmBackground: '#0f0f0f',
        mdmCard: '#1a1a1a',
        mdmBorder: '#2a2a2a',
        mdmMuted: '#1f1f1f',
        mdmAccent: '#251810',
    },
} as const

/**
 * Spacing scale (in pixels)
 * Based on 4px base unit
 */
export const spacing = {
    xs: '4px',
    sm: '8px',
    md: '16px',
    lg: '24px',
    xl: '32px',
    '2xl': '48px',
    '3xl': '64px',
} as const

/**
 * Typography scale
 */
export const typography = {
    fontFamily: {
        sans: [
            'system-ui',
            '-apple-system',
            'BlinkMacSystemFont',
            'Segoe UI',
            'Roboto',
            'sans-serif',
        ],
        mono: ['Menlo', 'Monaco', 'Courier New', 'monospace'],
    },
    fontSize: {
        xs: '0.75rem', // 12px
        sm: '0.875rem', // 14px
        base: '1rem', // 16px
        lg: '1.125rem', // 18px
        xl: '1.25rem', // 20px
        '2xl': '1.5rem', // 24px
        '3xl': '1.875rem', // 30px
        '4xl': '2.25rem', // 36px
    },
    fontWeight: {
        normal: 400,
        medium: 500,
        semibold: 600,
        bold: 700,
    },
    lineHeight: {
        tight: 1.25,
        normal: 1.5,
        relaxed: 1.75,
    },
} as const

/**
 * Border radius scale
 */
export const borderRadius = {
    none: '0',
    sm: '0.125rem', // 2px
    md: '0.25rem', // 4px
    lg: '0.5rem', // 8px
    xl: '0.75rem', // 12px
    '2xl': '1rem', // 16px
    full: '9999px',
} as const

/**
 * Shadow/elevation scale
 */
export const shadows = {
    sm: '0 1px 2px 0 rgba(0, 0, 0, 0.05)',
    md: '0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06)',
    lg: '0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -2px rgba(0, 0, 0, 0.05)',
    xl: '0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04)',
    '2xl': '0 25px 50px -12px rgba(0, 0, 0, 0.25)',
    none: 'none',
} as const

/**
 * Transition durations
 */
export const transitions = {
    fast: '150ms',
    normal: '200ms',
    slow: '300ms',
    slower: '500ms',
} as const

/**
 * Z-index scale
 */
export const zIndex = {
    base: 0,
    dropdown: 1000,
    sticky: 1020,
    fixed: 1030,
    modalBackdrop: 1040,
    modal: 1050,
    popover: 1060,
    tooltip: 1070,
} as const
