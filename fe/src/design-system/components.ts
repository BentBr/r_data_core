/**
 * Component Style Utilities
 *
 * Standardized props and configurations for common component patterns.
 * Use these utilities to ensure consistent styling across the application.
 */

import { buttonTokens, inputTokens, badgeTokens, dialogTokens, formTokens } from './tokens'

/**
 * Button component configurations
 */
export const buttonConfigs = {
    /**
     * Primary button - main actions
     */
    primary: {
        color: buttonTokens.variants.primary.color,
        variant: buttonTokens.variants.primary.variant,
    },
    /**
     * Secondary button - secondary actions
     */
    secondary: {
        color: buttonTokens.variants.secondary.color,
        variant: buttonTokens.variants.secondary.variant,
    },
    /**
     * Outlined button - alternative style
     */
    outlined: {
        variant: buttonTokens.variants.outlined.variant,
    },
    /**
     * Text button - minimal style
     */
    text: {
        variant: buttonTokens.variants.text.variant,
    },
    /**
     * Destructive button - delete/dangerous actions
     */
    destructive: {
        color: buttonTokens.variants.destructive.color,
        variant: buttonTokens.variants.destructive.variant,
    },
} as const

/**
 * Button size configurations
 */
export const buttonSizes = {
    small: buttonTokens.sizes.small,
    default: buttonTokens.sizes.default,
    large: buttonTokens.sizes.large,
} as const

/**
 * Input field configurations
 */
export const inputConfig = {
    variant: inputTokens.variant,
    density: inputTokens.density,
    color: inputTokens.focusColor,
} as const

/**
 * Badge/Chip configurations
 */
export const badgeConfigs = {
    /**
     * Status badge colors
     */
    status: {
        success: badgeTokens.statusColors.success,
        error: badgeTokens.statusColors.error,
        warning: badgeTokens.statusColors.warning,
        info: badgeTokens.statusColors.info,
        default: badgeTokens.statusColors.default,
    },
    /**
     * Badge sizes
     */
    sizes: badgeTokens.sizes,
    /**
     * Badge variant
     */
    variant: badgeTokens.variant,
} as const

/**
 * Dialog configurations
 */
export const dialogConfigs = {
    maxWidths: dialogTokens.maxWidths,
    padding: dialogTokens.padding,
} as const

/**
 * Form layout utilities
 */
export const formConfig = {
    spacing: formTokens.spacing,
} as const

/**
 * Get status color for badges/chips
 */
export function getStatusColor(status: string): string {
    const statusLower = status.toLowerCase()
    if (statusLower.includes('success') || statusLower.includes('completed') || statusLower === 'active') {
        return badgeConfigs.status.success
    }
    if (statusLower.includes('error') || statusLower.includes('failed') || statusLower === 'inactive') {
        return badgeConfigs.status.error
    }
    if (statusLower.includes('warning') || statusLower.includes('pending')) {
        return badgeConfigs.status.warning
    }
    if (statusLower.includes('info') || statusLower.includes('processing')) {
        return badgeConfigs.status.info
    }
    return badgeConfigs.status.default
}

/**
 * Standard icon sizes
 */
export const iconSizes = {
    xs: 16,
    sm: 20,
    md: 24,
    lg: 28,
    xl: 32,
} as const

/**
 * Standard dialog max widths based on content type
 */
export function getDialogMaxWidth(type: 'small' | 'default' | 'form' | 'wide' = 'default'): string {
    return dialogConfigs.maxWidths[type]
}

