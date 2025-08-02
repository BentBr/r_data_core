# TypeScript Model Bindings for R Data Core

This document outlines the comprehensive approaches for maintaining type-safe communication between the Vue frontend and Rust backend.

## 🎯 **Current Implementation: Enhanced Manual Types with Zod**

### ✅ **What We Have Now**

1. **Runtime Type Validation** with Zod schemas
2. **Type-Safe API Client** with automatic validation  
3. **Exact Backend Model Matching** with proper field types
4. **Development-Time Safety** with TypeScript inference

### 📁 **File Structure**
```
src/
├── types/
│   ├── schemas.ts          # Zod schemas + inferred types
│   └── api.ts              # Legacy types (can be removed)
├── api/
│   ├── typed-client.ts     # Type-safe HTTP client
│   └── http-client.ts      # Legacy client
```

### 🔍 **Example Usage**

```typescript
import { typedHttpClient } from '@/api/typed-client'

// ✅ Fully type-safe with runtime validation
const entityDefinitions = await typedHttpClient.getEntityDefinitions()
//    ^? EntityDefinition[] - TypeScript knows the exact type

// ✅ Runtime validation catches backend changes
try {
  const definition = await typedHttpClient.getEntityDefinition(uuid)
} catch (error) {
  // Will catch both HTTP errors AND schema mismatches
  console.error('API call failed:', error.message)
}
```

### 💪 **Benefits**

1. **Runtime Safety**: Catches backend changes immediately
2. **Type Safety**: Full TypeScript inference and checking
3. **IDE Support**: Perfect autocomplete and error detection
4. **Validation**: Ensures data matches expected format
5. **Error Handling**: Clear messages when schemas mismatch
6. **Maintainable**: Single source of truth for types

### 📝 **Schema Example**

```typescript
// schemas.ts
export const EntityDefinitionSchema = z.object({
    uuid: z.string().uuid(),
    entity_type: z.string(),
    display_name: z.string(),
    field_definitions: z.array(FieldDefinitionSchema),
    created_at: z.string().datetime(),
    // ... matches Rust struct exactly
})

// TypeScript type inferred automatically
export type EntityDefinition = z.infer<typeof EntityDefinitionSchema>
```

## 🚀 **Option 2: OpenAPI Code Generation (Future)**

### 📋 **Setup Steps** (when backend is accessible)

```bash
# Install OpenAPI tools
npm install -D openapi-typescript @apidevtools/swagger-parser

# Generate types from your backend
npx openapi-typescript http://rdatacore.docker/admin/api/docs/openapi.json -o src/types/generated.ts

# Add to package.json scripts
"generate-types": "openapi-typescript http://rdatacore.docker/admin/api/docs/openapi.json -o src/types/generated.ts"
```

### 💡 **Benefits**
- **Automatic synchronization** with backend changes
- **Zero manual maintenance** of types  
- **100% accuracy** - generated from actual API spec
- **Includes all endpoints** automatically

### ⚠️ **Considerations**
- Requires running backend during development
- Generated types can be verbose
- Less control over type structure
- Needs build process integration

## 🔧 **Option 3: Rust-to-TypeScript Code Generation**

### 🛠️ **Using typescript-rs** (Rust side)

Add to your Rust `Cargo.toml`:
```toml
[dependencies]
typescript-rs = "0.1"
serde = { version = "1.0", features = ["derive"] }
```

Annotate your Rust structs:
```rust
use typescript_rs::TypeScript;

#[derive(Serialize, Deserialize, TypeScript)]
#[serde(rename_all = "camelCase")]
#[typescript(export, export_to = "../fe/src/types/generated/")]
pub struct EntityDefinition {
    pub uuid: Uuid,
    pub entity_type: String,
    pub display_name: String,
    // ... rest of fields
}
```

Generate types:
```bash
# In Rust project
cargo test  # Generates TypeScript files
```

### 💪 **Benefits**
- **Direct from source** - no intermediary formats
- **Type-safe both ways** - Rust traits ensure consistency  
- **Minimal setup** once configured
- **Perfect field name mapping** with serde annotations

## 📊 **Comparison Matrix**

| Approach | Setup | Maintenance | Accuracy | Runtime Safety | Automation |
|----------|-------|-------------|-----------|----------------|------------|
| **Zod Schemas** | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐ |
| **OpenAPI Gen** | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐⭐⭐ |
| **Rust->TS Gen** | ⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐⭐ |

## 🎯 **Recommended Development Workflow**

### **Phase 1: Current (Zod Schemas)** ✅
- Use our current Zod-based approach
- Perfect for rapid development
- Runtime validation catches issues early
- Full TypeScript safety

### **Phase 2: Add OpenAPI Generation** (When backend is stable)
- Keep Zod for runtime validation
- Add OpenAPI generation for comprehensive coverage
- Use generated types as source of truth
- Zod schemas for critical runtime validation

### **Phase 3: Production Optimization** (Optional)
- Consider Rust->TypeScript generation for performance
- Evaluate if runtime validation is needed everywhere
- Optimize bundle size if needed

## 🔨 **Current Usage Patterns**

### **In Components**
```vue
<script setup lang="ts">
import { typedHttpClient } from '@/api/typed-client'
import type { EntityDefinition } from '@/types/schemas'

// ✅ Full type safety + runtime validation
const entityDefinitions = ref<EntityDefinition[]>([])

onMounted(async () => {
  try {
    entityDefinitions.value = await typedHttpClient.getEntityDefinitions()
  } catch (error) {
    // Handles both network and validation errors
    console.error('Failed to load entity definitions:', error)
  }
})
</script>
```

### **In Stores (Pinia)**
```typescript
export const useEntityDefinitionStore = defineStore('entityDefinitions', () => {
  const definitions = ref<EntityDefinition[]>([])
  
  const fetchDefinitions = async () => {
    // ✅ Automatic validation and type inference
    definitions.value = await typedHttpClient.getEntityDefinitions()
  }
  
  return { definitions, fetchDefinitions }
})
```

## 🚀 **Next Steps**

1. **Migrate existing API calls** to use `typedHttpClient`
2. **Add more schemas** for remaining backend models
3. **Set up OpenAPI generation** when backend is accessible
4. **Update PLAN.md** to reflect TypeScript binding completion

## 📈 **Benefits Summary**

✅ **Developer Experience**: Perfect IDE support with autocomplete and error checking  
✅ **Runtime Safety**: Catches backend changes before they break the UI  
✅ **Type Safety**: Compile-time guarantees about data structure  
✅ **Maintainability**: Single source of truth for data models  
✅ **Error Handling**: Clear error messages when things go wrong  
✅ **Future-Proof**: Easy migration to generated types when ready  

The current Zod-based approach provides **the best balance** of safety, maintainability, and developer experience for the R Data Core admin interface! 
