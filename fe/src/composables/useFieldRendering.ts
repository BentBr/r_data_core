import { useTranslations } from './useTranslations'
import type { FieldDefinition } from '@/types/schemas'

/**
 * Field rendering utilities
 *
 * Note: All returned Vuetify components automatically use standardized styling
 * from the design system defaults configured in fe/src/design-system/theme.ts
 */
export function useFieldRendering() {
    const { t } = useTranslations()

    /**
     * Maps field types to Vuetify components
     * All components use standardized styling via Vuetify defaults
     */
    const getFieldComponent = (fieldType: string): string => {
        const componentMap: Record<string, string> = {
            String: 'v-text-field',
            Text: 'v-textarea',
            Wysiwyg: 'v-textarea',
            Integer: 'v-text-field',
            Float: 'v-text-field',
            Boolean: 'v-checkbox',
            Date: 'v-text-field',
            DateTime: 'v-text-field',
            Time: 'v-text-field',
            Email: 'v-text-field',
            Url: 'v-text-field',
            File: 'v-file-input',
            Image: 'v-file-input',
            Json: 'v-textarea',
            Object: 'v-textarea',
            Array: 'v-textarea',
            Uuid: 'v-text-field',
            ManyToOne: 'v-select',
            ManyToMany: 'v-combobox',
            Select: 'v-select',
            MultiSelect: 'v-combobox',
        }
        return componentMap[fieldType] || 'v-text-field'
    }

    /**
     * Gets field validation rules based on field definition
     */
    /**
     * Validation rule function type
     */
    type ValidationRule = (value: unknown) => boolean | string

    const getFieldRules = (field: FieldDefinition): ValidationRule[] => {
        const rules: ValidationRule[] = []

        if (field.required) {
            rules.push(v => !!v || `${field.display_name || field.name} is required`)
        }

        if (field.constraints?.min !== undefined) {
            const minVal = Number(field.constraints.min)
            rules.push(
                v => !v || (typeof v === 'number' && v >= minVal) || `Minimum value is ${minVal}`
            )
        }

        if (field.constraints?.max !== undefined) {
            const maxVal = Number(field.constraints.max)
            rules.push(
                v => !v || (typeof v === 'number' && v <= maxVal) || `Maximum value is ${maxVal}`
            )
        }

        if (field.constraints?.pattern) {
            const pattern = field.constraints.pattern as string
            rules.push(
                v =>
                    !v ||
                    new RegExp(pattern).test(String(v)) ||
                    `Invalid format for ${field.display_name || field.name}`
            )
        }

        return rules
    }

    /**
     * Gets the icon for a field type
     */
    const getFieldIcon = (fieldType: string): string => {
        const iconMap: Record<string, string> = {
            String: 'type',
            Text: 'file-text',
            Wysiwyg: 'file-edit',
            Integer: 'hash',
            Float: 'hash',
            Boolean: 'check-square',
            Date: 'calendar',
            DateTime: 'calendar-clock',
            Time: 'clock',
            Email: 'mail',
            Url: 'link',
            File: 'file',
            Image: 'image',
            Json: 'code',
            Object: 'code',
            Array: 'list',
            Uuid: 'hash',
            ManyToOne: 'link',
            ManyToMany: 'link-2',
            Select: 'list-checks',
            MultiSelect: 'list-checks',
        }
        return iconMap[fieldType] || 'type'
    }

    /**
     * Formats a field value for display based on its type
     */
    const formatFieldValue = (value: unknown, fieldType: string): string => {
        if (value === null || value === undefined) {
            return t('common.empty')
        }

        switch (fieldType) {
            case 'Boolean':
                return value === true ? t('common.yes') : t('common.no')
            case 'Date':
            case 'DateTime':
                return new Date(value as string).toLocaleDateString()
            case 'Time':
                return new Date(`2000-01-01T${value}`).toLocaleTimeString()
            case 'Json':
            case 'Object':
                return typeof value === 'object' ? JSON.stringify(value) : String(value)
            case 'Array':
                return Array.isArray(value) ? `[${value.length} items]` : String(value)
            default:
                return String(value)
        }
    }

    /**
     * Get the input type for HTML inputs based on field type
     */
    const getInputType = (fieldType: string): string => {
        const typeMap: Record<string, string> = {
            String: 'text',
            Text: 'text',
            Wysiwyg: 'text',
            Integer: 'number',
            Float: 'number',
            Boolean: 'checkbox',
            Date: 'date',
            DateTime: 'datetime-local',
            Time: 'time',
            Email: 'email',
            Url: 'url',
            File: 'file',
            Image: 'file',
        }
        return typeMap[fieldType] || 'text'
    }

    /**
     * Checks if a field type requires JSON parsing before sending to API
     */
    const isJsonFieldType = (fieldType: string): boolean => {
        return ['Json', 'Object', 'Array'].includes(fieldType)
    }

    /**
     * Parses JSON field values from strings to objects
     * Returns the original value if parsing fails or if value is already an object
     */
    const parseJsonFieldValue = (
        value: unknown,
        fieldType: string
    ): { parsed: unknown; error: string | null } => {
        if (!isJsonFieldType(fieldType)) {
            return { parsed: value, error: null }
        }

        // If value is already an object/array, return as-is
        if (typeof value === 'object') {
            return { parsed: value, error: null }
        }

        // If value is empty string, return as-is
        if (value === '') {
            return { parsed: value, error: null }
        }

        // Try to parse string as JSON (value is string at this point)
        if (typeof value === 'string') {
            try {
                const parsed = JSON.parse(value) as unknown
                // Validate the parsed result matches the expected type
                if (fieldType === 'Array' && !Array.isArray(parsed)) {
                    return { parsed: value, error: 'Must be a valid JSON array' }
                }
                if (
                    (fieldType === 'Object' || fieldType === 'Json') &&
                    (typeof parsed !== 'object' || Array.isArray(parsed) || parsed === null)
                ) {
                    return { parsed: value, error: 'Must be a valid JSON object' }
                }
                return { parsed, error: null }
            } catch {
                return { parsed: value, error: 'Must be valid JSON' }
            }
        }

        return { parsed: value, error: null }
    }

    /**
     * Stringifies JSON field values for display in textarea
     */
    const stringifyJsonFieldValue = (value: unknown, fieldType: string): string => {
        if (!isJsonFieldType(fieldType)) {
            return String(value ?? '')
        }

        if (value === null || value === undefined) {
            return ''
        }

        if (typeof value === 'object') {
            return JSON.stringify(value, null, 2)
        }

        return String(value)
    }

    return {
        getFieldComponent,
        getFieldRules,
        getFieldIcon,
        formatFieldValue,
        getInputType,
        isJsonFieldType,
        parseJsonFieldValue,
        stringifyJsonFieldValue,
    }
}
