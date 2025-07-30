# Translation System Documentation

## Overview
The translation system is built using a Vue 3 composable pattern with persistence and real-time language switching.

## Locale Storage & Access

### Where is the locale stored?
The current language preference is stored in **two places**:

1. **localStorage** (Browser persistence)
   - Key: `'preferred-language'`
   - Values: `'en'` | `'de'`
   - Persists across browser sessions

2. **Vue Reactive State** (Runtime state)
   - Managed by the `useTranslations` composable
   - Automatically synced with localStorage

### How to access the current locale?

#### Method 1: Using the composable (recommended)
```typescript
import { useTranslations } from '@/composables/useTranslations'

const { currentLanguage, setLanguage } = useTranslations()

// Get current language
console.log(currentLanguage.value) // 'en' | 'de'

// Change language
await setLanguage('de')
```

#### Method 2: Direct localStorage access
```typescript
// Get current language
const currentLang = localStorage.getItem('preferred-language') || 'en'

// Set language (not recommended - use composable instead)
localStorage.setItem('preferred-language', 'de')
```

## Translation Functions

### Basic Translation
```typescript
const { t } = useTranslations()

// Translate with key
const title = t('auth.login.title') // "Sign In" or "Anmelden"

// Translate with fallback
const message = t('unknown.key', 'Default message')
```

### Error Translation
```typescript
const { translateError } = useTranslations()

// Automatically maps backend errors to user-friendly translations
const friendlyError = translateError('Invalid credentials')
// Returns: "Invalid username or password" (EN) or "Ungültiger Benutzername oder Passwort" (DE)
```

## Available Languages
- **English (`'en'`)**: Default fallback language
- **German (`'de'`)**: Secondary language

## File Structure
```
fe/
├── translations/
│   ├── en.json          # English translations
│   └── de.json          # German translations
├── src/composables/
│   └── useTranslations.ts  # Main translation composable
└── src/components/common/
    └── LanguageSwitch.vue   # Language switcher component
```

## Translation Keys Structure
```json
{
  "auth": {
    "login": {
      "title": "Sign In",
      "username": "Username",
      "errors": {
        "invalid_credentials": "Invalid username or password"
      }
    }
  },
  "navigation": {
    "dashboard": "Dashboard",
    "logout": "Logout"
  },
  "general": {
    "language": {
      "switch": "Switch Language",
      "current": "Current"
    }
  }
}
```

## Adding New Translations

1. **Add to translation files**:
   ```json
   // fe/translations/en.json
   "new_section": {
     "new_key": "English text"
   }
   
   // fe/translations/de.json  
   "new_section": {
     "new_key": "German text"
   }
   ```

2. **Use in components**:
   ```vue
   <template>
     <div>{{ t('new_section.new_key') }}</div>
   </template>
   
   <script setup>
   import { useTranslations } from '@/composables/useTranslations'
   const { t } = useTranslations()
   </script>
   ```

## Language Switching
The language switch is available in:
- **Login page**: Top-right corner of login card
- **Main app**: App bar header next to theme toggle

Language changes are:
- ✅ Applied immediately (no page refresh)
- ✅ Persisted in localStorage  
- ✅ Applied to all components using translations
- ✅ Include proper country flags (🇬🇧 UK, 🇩🇪 Germany)

## Error Message Translation
Backend error messages are automatically translated using pattern matching:
- "invalid credentials" → `auth.login.errors.invalid_credentials`
- "username required" → `auth.login.errors.username_required`
- "network error" → `auth.login.errors.network_error`

## Component Integration Example
```vue
<template>
  <div>
    <!-- Basic translation -->
    <h1>{{ t('navigation.dashboard') }}</h1>
    
    <!-- With fallback -->
    <p>{{ t('unknown.key', 'Default text') }}</p>
    
    <!-- Error translation -->
    <div v-if="error">{{ translateError(error) }}</div>
    
    <!-- Language switcher -->
    <LanguageSwitch />
  </div>
</template>

<script setup>
import { useTranslations } from '@/composables/useTranslations'
import LanguageSwitch from '@/components/common/LanguageSwitch.vue'

const { t, translateError, currentLanguage } = useTranslations()

// React to language changes
watch(currentLanguage, (newLang) => {
  console.log('Language changed to:', newLang)
})
</script>
```