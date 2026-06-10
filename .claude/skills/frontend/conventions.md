---
name: frontend-conventions
description: Vue3, TypeScript, Vite, Vuetify, and Pinia conventions for the admin frontend (fe/)
---

# Frontend Conventions

## Tech Stack

- **Framework**: Vue 3 with Composition API (`<script setup lang="ts">`)
- **UI Library**: Vuetify 3
- **State Management**: Pinia
- **Routing**: Vue Router 4
- **Validation**: Zod
- **Build**: Vite 6
- **Testing**: Vitest (unit), Playwright (e2e)
- **Linting**: ESLint + Prettier

## Directory Structure

```
fe/
├── src/
│   ├── api/           # API client code
│   ├── components/    # Reusable Vue components
│   ├── composables/   # Vue 3 composables
│   ├── design-system/ # Design system components
│   ├── layouts/       # Page layouts
│   ├── pages/         # Page components
│   ├── stores/        # Pinia state stores
│   ├── types/         # TypeScript type definitions
│   ├── utils/         # Utility functions
│   └── router/        # Route definitions
├── e2e/               # Playwright end-to-end tests
└── translations/      # i18n files
```

## Running Commands

**Never run `npm` locally.** All frontend commands must run inside the Docker `node` container.

Use `rdt` (preferred) or `docker compose exec node`:

```bash
rdt test-fe                            # Run vitest via Docker
rdt lint                               # Run ESLint + Prettier via Docker
rdt test-e2e                           # Run Playwright E2E tests via Docker
docker compose exec node npm run dev   # Only if no rdt alias exists
```

## Key Patterns

- TypeScript strict mode
- Components use PascalCase naming
- Composables use `use-` prefix with kebab-case
- API client in `src/api/` wraps HTTP calls
- Pinia stores in `src/stores/` for state management
