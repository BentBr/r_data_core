import { z } from 'zod'

// Common table/action schemas
export const TableActionSchema = z.object({
    icon: z.string(),
    color: z.string().optional(),
    tooltip: z.string().optional(),
    disabled: z.boolean().optional(),
    loading: z.boolean().optional(),
})

export const TableColumnSchema = z.object({
    key: z.string(),
    title: z.string(),
    sortable: z.boolean().optional(),
    align: z.enum(['start', 'center', 'end']).optional(),
    width: z.string().optional(),
    fixed: z.boolean().optional(),
})

// Tree node type definition
export interface TreeNode {
    id: string
    title: string
    icon?: string
    color?: string
    children?: TreeNode[]
    expanded?: boolean
    selected?: boolean
    disabled?: boolean
    entity_type?: string
    uuid?: string
    display_name?: string
    published?: boolean
    hasChildren?: boolean
    path?: string
}

// Tree view schemas - recursive type with explicit base schema
const TreeNodeBaseSchema = z.object({
    id: z.string(),
    title: z.string(),
    icon: z.string().optional(),
    color: z.string().optional(),
    expanded: z.boolean().optional(),
    selected: z.boolean().optional(),
    disabled: z.boolean().optional(),
    entity_type: z.string().optional(),
    uuid: z.string().optional(),
    display_name: z.string().optional(),
    published: z.boolean().optional(),
    hasChildren: z.boolean().optional(),
    path: z.string().optional(),
})

export const TreeNodeSchema: z.ZodType<TreeNode> = TreeNodeBaseSchema.extend({
    children: z.lazy(() => z.array(TreeNodeSchema)).optional(),
})

// Snackbar schemas
export const SnackbarConfigSchema = z.object({
    message: z.string(),
    color: z.enum(['success', 'error', 'warning', 'info']).optional(),
    timeout: z.number().optional(),
    persistent: z.boolean().optional(),
})

// Dialog schemas
export const DialogConfigSchema = z.object({
    title: z.string(),
    width: z.string().optional(),
    persistent: z.boolean().optional(),
    maxWidth: z.string().optional(),
})

// Form field schemas
export const FormFieldSchema = z.object({
    name: z.string(),
    label: z.string(),
    type: z.enum(['text', 'textarea', 'select', 'switch', 'number', 'date', 'email', 'password']),
    required: z.boolean().optional(),
    rules: z.array(z.string()).optional(),
    options: z.array(z.object({ value: z.string(), label: z.string() })).optional(),
    placeholder: z.string().optional(),
    hint: z.string().optional(),
    disabled: z.boolean().optional(),
})

// Type exports
export type TableAction = z.infer<typeof TableActionSchema>
export type TableColumn = z.infer<typeof TableColumnSchema>
// TreeNode is exported as an interface above
export type SnackbarConfig = z.infer<typeof SnackbarConfigSchema>
export type DialogConfig = z.infer<typeof DialogConfigSchema>
export type FormField = z.infer<typeof FormFieldSchema>
