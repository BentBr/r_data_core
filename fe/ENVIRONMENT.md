# Environment Variables Configuration

## 🔧 **Setup**

Environment variables are configured in the `compose.yaml` file for the Docker development environment.

1. **Edit environment variables:**
   ```bash
   # Edit compose.yaml in the project root
   nano ../compose.yaml
   ```

2. **Restart the node service:**
   ```bash
   docker compose stop node
   docker compose up -d node
   ```

## 📋 **Available Variables**

### **API Configuration**
- `VITE_API_BASE_URL` - Backend API base URL
  - Docker: `http://rdatacore.docker`
  - Production: `https://your-api-domain.com`

### **Development Settings**
- `VITE_DEV_MODE` - Enable development features
  - Values: `true` | `false`
  - Default: `false`

- `VITE_ENABLE_API_LOGGING` - Console API request/response logging
  - Values: `true` | `false`
  - Default: `false`

### **UI Configuration**
- `VITE_DEFAULT_PAGE_SIZE` - Default pagination size
  - Default: `20`
  - Used in API calls when no limit specified

- `VITE_MAX_PAGE_SIZE` - Maximum allowed page size
  - Default: `100`

### **Authentication**
- `VITE_TOKEN_REFRESH_BUFFER` - Minutes before token expiry to refresh
  - Default: `5`

### **Feature Flags**
- `VITE_ENABLE_API_KEY_MANAGEMENT` - Show/hide API key management
  - Values: `true` | `false`
  - Default: `true`

- `VITE_ENABLE_USER_MANAGEMENT` - Show/hide user management
  - Values: `true` | `false`
  - Default: `true`

- `VITE_ENABLE_SYSTEM_METRICS` - Show/hide system metrics
  - Values: `true` | `false`
  - Default: `true`

## 💻 **Usage in Code**

### **Type-Safe Environment Access**
```typescript
import { env, features } from '@/env-check'

// Environment variables
console.log(env.apiBaseUrl)     // string
console.log(env.devMode)        // boolean
console.log(env.defaultPageSize) // number

// Feature flags
if (features.apiKeyManagement) {
  // Show API key management UI
}
```

### **Direct Access (Not Recommended)**
```typescript
// ❌ Not type-safe, can be undefined
const apiUrl = import.meta.env.VITE_API_BASE_URL

// ✅ Better - type-safe with defaults
const apiUrl = env.apiBaseUrl
```

### **In Components**
```vue
<script setup lang="ts">
import { env } from '@/env-check'

// Use environment values
const apiUrl = env.apiBaseUrl
const isDev = env.isDevelopment
</script>

<template>
  <div v-if="isDev">
    Development Mode: API at {{ apiUrl }}
  </div>
</template>
```

## 🔍 **Environment Variable Checking**

The app automatically logs environment variables in development mode. Check your browser console to see loaded values.

### **Manual Check**
```typescript
import { checkEnvironmentVariables } from '@/env-check'

// View all environment variables
const envVars = checkEnvironmentVariables()
console.table(envVars)
```

## 🌍 **Environment Configuration**

Environment variables are configured in the `compose.yaml` file under the `node` service's `environment` section.

## 📱 **Current Docker Configuration**

The following environment variables are configured in `compose.yaml`:

```yaml
# compose.yaml - node service environment
environment:
  - VIRTUAL_HOST=.admin.rdatacore.docker
  - MAIN_SERVICE=node
  # Frontend Environment Variables
  - VITE_API_BASE_URL=http://rdatacore.docker
  - VITE_DEV_MODE=true
  - VITE_ENABLE_API_LOGGING=true
  - VITE_TOKEN_REFRESH_BUFFER=5
  - VITE_DEFAULT_PAGE_SIZE=20
  - VITE_MAX_PAGE_SIZE=100
  - VITE_ENABLE_API_KEY_MANAGEMENT=true
  - VITE_ENABLE_USER_MANAGEMENT=true
  - VITE_ENABLE_SYSTEM_METRICS=true
  - VITE_ENABLE_PRODUCTION_SOURCEMAPS=false
```

### **For Production Deployment**
Create a separate production compose file or override environment variables:

```yaml
# compose.production.yaml
services:
  node:
    environment:
      - VITE_API_BASE_URL=https://api.yourdomain.com
      - VITE_DEV_MODE=false
      - VITE_ENABLE_API_LOGGING=false
      - VITE_ENABLE_PRODUCTION_SOURCEMAPS=false
```

## 🚀 **Best Practices**

1. **Environment variables are in `compose.yaml`** - version controlled and shared
2. **Always provide defaults** in your code
3. **Use the `env` helper** for type safety
4. **Document new variables** in this file and `compose.yaml`
5. **Use feature flags** for conditional functionality
6. **Prefix with `VITE_`** - required for client-side access
7. **Restart node service** after changing environment variables

## 🔒 **Security Notes**

- ⚠️ **Client-side variables are public** - never put secrets in `VITE_*` variables
- ✅ **API keys should be handled server-side** 
- ✅ **Use environment variables for configuration**, not secrets
- ✅ **Sensitive data belongs in server environment** only

## 🛠️ **Development Workflow**

1. Edit environment variables in `compose.yaml`
2. Restart the node service: `docker compose stop node && docker compose up -d node`
3. Check console for loaded values in the browser
4. Update code to use `env` helper for type safety
5. Document any new variables in this file

### **Adding New Environment Variables**

1. **Add to `compose.yaml`:**
   ```yaml
   - VITE_NEW_FEATURE=true
   ```

2. **Add to `env-check.ts`:**
   ```typescript
   export const env = {
     // ... existing variables
     newFeature: import.meta.env.VITE_NEW_FEATURE === 'true',
   }
   ```

3. **Restart node service:**
   ```bash
   docker compose stop node && docker compose up -d node
   ```

Environment variables are now fully integrated with Docker Compose, type safety, and runtime validation! 🎉 