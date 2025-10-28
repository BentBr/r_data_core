import { useTranslations } from './useTranslations'
import type { FieldDefinition } from '@/types/schemas'

export function useFieldRendering() {
    const { t } = useTranslations()

    /**
     * Maps field types to Vuetify components
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
    const getFieldRules = (field: FieldDefinition) => {
        const rules: ((value: unknown) => boolean | string)[] = []

        if (field.required) {
            rules.push(v => !!v || `${field.display_name ?? field.name} is required`)
        }

        if (field.constraints?.min !== undefined) {
            rules.push(
                v =>
                    !v ||
                    (typeof v === 'number' && v >= field.constraints!.min!) ||
                    `Minimum value is ${field.constraints!.min}`
            )
        }

        if (field.constraints?.max !== undefined) {
            rules.push(
                v =>
                    !v ||
                    (typeof v === 'number' && v <= field.constraints!.max!) ||
                    `Maximum value is ${field.constraints!.max}`
            )
        }

        if (field.constraints?.pattern) {
            const pattern = field.constraints.pattern as string
            rules.push(
                v =>
                    !v ||
                    new RegExp(pattern).test(String(v)) ||
                    `Invalid format for ${field.display_name ?? field.name}`
            )
        }

        return rules
    }

    /**
     * Gets the icon for a field type
     */
    const getFieldIcon = (fieldType: string): string => {
        const iconMap: Record<string, string> = {
            String: 'mdi-text',
            Text: 'mdi-text-box',
            Wysiwyg: 'mdi-text-box-edit',
            Integer: 'mdi-numeric',
            Float: 'mdi-decimal',
            Boolean: 'mdi-checkbox-marked',
            Date: 'mdi-calendar',
            DateTime: 'mdi-calendar-clock',
            Time: 'mdi-clock',
            Email: 'mdi-email',
            Url: 'mdi-link',
            File: 'mdi-file',
            Image: 'mdi-image',
            Json: 'mdi-code-json',
            Object: 'mdi-code-json',
            Array: 'mdi-code-array',
            Uuid: 'mdi-identifier',
            ManyToOne: 'mdi-link-variant',
            ManyToMany: 'mdi-link-variant-plus',
            Select: 'mdi-format-list-bulleted',
            MultiSelect: 'mdi-format-list-bulleted-square',
        }
        return iconMap[fieldType] || 'mdi-text'
    }

    /**
     * Formats a field value for display based on its type
     */
    const formatFieldValue = (value: unknown, fieldType: string): string => {
        if (value === null ?? value === undefined) {
            return t('common.empty') || 'Empty'
        }

        switch (fieldType) {
            case 'Boolean':
                return value ? t('common.yes') || 'Yes' : t('common.no') || 'No'
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

    return {
        getFieldComponent,
        getFieldRules,
        getFieldIcon,
        formatFieldValue,
        getInputType,
    }
}
