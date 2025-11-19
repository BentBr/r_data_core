/**
 * Type formatting utilities to convert values to proper types based on field definitions
 */

export type FieldType =
    | 'String'
    | 'Text'
    | 'Wysiwyg'
    | 'Integer'
    | 'Float'
    | 'Boolean'
    | 'Date'
    | 'DateTime'
    | 'Object'
    | 'Array'
    | 'Uuid'
    | 'ManyToOne'
    | 'ManyToMany'
    | 'Select'
    | 'MultiSelect'
    | 'Image'
    | 'File'

/**
 * Format a value to the proper type based on field type
 * Handles string-to-type conversions (e.g., "false" -> boolean false)
 */
export function formatValueToType(value: any, fieldType: FieldType): any {
    if (value === null || value === undefined || value === '') {
        return null
    }

    switch (fieldType) {
        case 'Boolean':
            if (typeof value === 'boolean') {
                return value
            }
            if (typeof value === 'string') {
                const lower = value.toLowerCase().trim()
                return lower === 'true' || lower === '1' || lower === 'yes' || lower === 'on'
            }
            if (typeof value === 'number') {
                return value !== 0
            }
            return false

        case 'Integer':
            if (typeof value === 'number') {
                return Math.floor(value)
            }
            if (typeof value === 'string') {
                const parsed = parseInt(value, 10)
                return isNaN(parsed) ? null : parsed
            }
            return null

        case 'Float':
            if (typeof value === 'number') {
                return value
            }
            if (typeof value === 'string') {
                const parsed = parseFloat(value)
                return isNaN(parsed) ? null : parsed
            }
            return null

        case 'Date':
        case 'DateTime':
            // Keep as string for date/datetime (ISO format expected)
            return typeof value === 'string' ? value : null

        case 'Object':
        case 'Array':
            // If already an object/array, return as-is
            if (typeof value === 'object') {
                return value
            }
            // Try to parse JSON string
            if (typeof value === 'string') {
                try {
                    return JSON.parse(value)
                } catch {
                    return null
                }
            }
            return null

        default:
            // String, Text, Wysiwyg, Uuid, Select, etc. - keep as string
            return typeof value === 'string' ? value : String(value)
    }
}

/**
 * Format field data object by applying type formatting to all fields based on field definitions
 */
export function formatFieldData(
    fieldData: Record<string, any>,
    fieldDefinitions: Array<{ name: string; field_type: FieldType }>
): Record<string, any> {
    const formatted: Record<string, any> = {}
    const fieldTypeMap = new Map<string, FieldType>()

    // Build field type map
    for (const fieldDef of fieldDefinitions) {
        fieldTypeMap.set(fieldDef.name, fieldDef.field_type)
    }

    // Format each field value
    for (const [key, value] of Object.entries(fieldData)) {
        const fieldType = fieldTypeMap.get(key)
        if (fieldType) {
            formatted[key] = formatValueToType(value, fieldType)
        } else {
            // Unknown field, keep as-is
            formatted[key] = value
        }
    }

    return formatted
}
