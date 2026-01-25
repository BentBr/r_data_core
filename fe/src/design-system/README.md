# Design System Documentation

This directory contains the unified design system for R Data Core, extracted from the Figma design
system: https://www.figma.com/make/TfyVgKu2VFjIRq4frfI3CM/RDataCore

## Overview

The design system provides consistent styling, components, and patterns across the entire application. All UI elements
follow the same design principles and use standardized tokens.

## Files

- **`tokens.ts`** - Design tokens (colors, spacing, typography, component-specific tokens)
- **`theme.ts`** - Vuetify theme configuration and component defaults
- **`components.ts`** - Component style utilities and helper functions
- **`index.ts`** - Central export point

## Design Tokens

### Colors

Colors are defined for both light and dark themes in `tokens.ts`:

- **Brand Colors**: Primary orange (`#ff6b00` light, `#ff8533` dark)
- **Background Colors**: Card and surface colors
- **Accent Colors**: Secondary and accent variations
- **Informational Colors**: Success, error, warning, info
- **Chart Colors**: 5-color palette for data visualization

### Spacing

Based on 4px base unit:

- `xs`: 4px
- `sm`: 8px
- `md`: 16px
- `lg`: 24px
- `xl`: 32px
- `2xl`: 48px
- `3xl`: 64px

### Typography

- **Font Family**: System fonts (sans-serif) and monospace
- **Font Sizes**: xs (12px) to 4xl (36px)
- **Font Weights**: normal (400), medium (500), semibold (600), bold (700)
- **Line Heights**: tight (1.25), normal (1.5), relaxed (1.75)

### Border Radius

- `sm`: 2px
- `md`: 4px
- `lg`: 8px (default for inputs and buttons)
- `xl`: 12px (default for cards)
- `2xl`: 16px
- `full`: 9999px (for badges/chips)

## Component Standards

### Buttons

**Variants:**

- **Primary**: `color="primary"`, `variant="flat"` - Main actions
- **Secondary**: `color="secondary"`, `variant="outlined"` - Secondary actions
- **Text**: `variant="text"` - Minimal style
- **Destructive**: `color="error"`, `variant="flat"` - Delete/dangerous actions

**Sizes:**

- `small`, `default`, `large`

**Example:**

```vue
<v-btn color="primary" variant="flat">Save</v-btn>
<v-btn color="secondary" variant="outlined">Cancel</v-btn>
<v-btn variant="text" color="mutedForeground">Close</v-btn>
```

### Input Fields

All input fields use standardized styling via Vuetify defaults:

- **Variant**: `outlined` (default)
- **Density**: `comfortable` (default)
- **Color**: `primary` for focus states
- **Border Radius**: 8px

**Components:**

- `v-text-field` - Text inputs
- `v-select` - Dropdowns
- `v-textarea` - Multi-line text
- `v-autocomplete` - Autocomplete
- `v-combobox` - Combobox
- `v-file-input` - File uploads

**Example:**

```vue
<v-text-field
    v-model="value"
    label="Name"
    variant="outlined"
/>
```

### Badges

Use the unified `Badge` component instead of `v-chip`:

```vue
<Badge status="success" size="small">Active</Badge>
<Badge status="error" size="small">Inactive</Badge>
<Badge color="primary" size="small">Custom</Badge>
```

**Props:**

- `status`: Auto-maps to color ('success', 'error', 'warning', 'info')
- `color`: Direct color override
- `size`: 'small', 'default', 'large'
- `variant`: 'flat' (default), 'outlined', 'text', etc.

### Icons

Use `SmartIcon` component with standard sizes:

```vue
<SmartIcon icon="user" size="sm" />
<SmartIcon icon="settings" size="md" />
<SmartIcon icon="database" size="lg" />
```

**Standard Sizes:**

- `xs`: 16px - Small inline icons, table actions
- `sm`: 20px - Button icons, form field icons
- `md`: 24px - Default size, most common use case
- `lg`: 28px - Page headers, prominent icons
- `xl`: 32px - Large display icons

### Dialogs

Dialogs use consistent styling:

**Max Widths:**

- `small`: 400px
- `default`: 600px
- `form`: 800px
- `wide`: 1200px

**Padding:**

- Title/Content: 24px (pa-6)
- Actions: 16px vertical, 24px horizontal (pa-4 px-6)

**Example:**

```vue
<v-dialog :max-width="getDialogMaxWidth('form')">
    <v-card>
        <v-card-title class="pa-6">Title</v-card-title>
        <v-card-text class="pa-6">Content</v-card-text>
        <v-card-actions class="pa-4 px-6">
            <v-spacer />
            <v-btn variant="text" color="mutedForeground">Cancel</v-btn>
            <v-btn color="primary" variant="flat">Confirm</v-btn>
        </v-card-actions>
    </v-card>
</v-dialog>
```

### Cards

- **Border Radius**: 12px (xl)
- **Elevation**: 2 (default)
- **Padding**: 24px (lg)

## Usage Guidelines

### Importing Design System

```typescript
import { colorTokens, spacing, typography } from '@/design-system'
import { buttonConfigs, inputConfig, badgeConfigs } from '@/design-system/components'
import { getDialogMaxWidth, getStatusColor } from '@/design-system/components'
```

### Component Defaults

Component defaults are automatically applied via Vuetify configuration in `main.ts`. You don't need to specify common
props unless you want to override them.

### Color Usage

- Use semantic colors (success, error, warning, info) for status indicators
- Use primary color for main actions and focus states
- Use muted colors for secondary text and borders
- Always ensure sufficient contrast for accessibility

### Spacing

Use spacing tokens consistently:

- Form fields: `mb-4` (16px) between fields
- Sections: `mt-6` (24px) or `mb-6` between sections
- Buttons: `gap-2` (8px) between button groups

### Icons

Always use `SmartIcon` component with standard size names (xs, sm, md, lg, xl) rather than pixel values for consistency.

## Examples

See the example designs in `/example_designs/` folder for visual reference:

- `input_form_controls.png` - Input field styling
- `buttons_elements.png` - Button variants
- `brand_background_colours.png` - Color palette
- `accent_ui-element_colours.png` - Accent colors
- `informational_colours.png` - Status colors
- `login_light.png` / `login_dark.png` - Login page styling
- `settings.png` - Settings page layout

## Maintenance

When adding new components or updating existing ones:

1. Use design tokens from `tokens.ts`
2. Follow component standards documented above
3. Use utility functions from `components.ts`
4. Ensure consistency with Figma design system
5. Test in both light and dark themes

