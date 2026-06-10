---
name: frontend
description: Admin frontend (fe/) — Vue3, TypeScript, Vite, Vuetify, Pinia, Playwright e2e. Component/state/infrastructure layering, generated-types boundary, EN/DE translations. Read by all frontend subagents.
color: blue
---

# Admin Frontend

**Path**: `fe/`
**Role**: Admin dashboard for managing entities, workflows, users, and system settings.

## Tech Stack

| Technology | Version | Purpose |
|-----------|---------|---------|
| Vue 3 | 3.x | UI framework (Composition API) |
| TypeScript | strict | Type safety |
| Vite | 6 | Build tool and dev server |
| Vuetify | 3.5 | Material Design component library |
| Pinia | latest | State management |
| Vue Router | 4.3 | Client-side routing |
| Zod | latest | Runtime validation |
| VueUse | latest | Composable utilities |
| Lucide Vue | latest | Icons |

## Directory Structure

```
fe/
├── src/
│   ├── api/           # API client wrappers
│   ├── components/    # Reusable Vue components
│   ├── composables/   # Vue 3 composables (use-* prefix)
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

**Never run `npm` (or `npx`) locally.** All frontend commands must run inside the Docker `node` container.

Use `rdt` (preferred) or `docker compose exec node`:

```bash
rdt test-fe                            # Vitest unit tests
rdt lint                               # ESLint + Prettier
rdt test-e2e                           # Playwright E2E browser tests
rdt test-e2e-report                    # Serve Playwright report
rdt clean-e2e                          # Remove E2E test data from DB
docker compose exec node npm run dev   # Only if no rdt alias exists
```

Runs in the `node` Docker container. Dev server accessible at the configured Docker hostname.

## Conventions

- Composition API with `<script setup lang="ts">`
- Components: PascalCase
- Composables: kebab-case with `use-` prefix
- TypeScript strict mode — avoid `any`
- See the `conventions.md` supporting doc for full details

## Layering (mirrors the subagent split)

- **ui** (`frontend-ui`) — `fe/src` components, views, SCSS, Vuetify wrappers.
- **state** (`frontend-state`) — Pinia stores, composables (`use*`), HTTP/API services.
- **infrastructure** (`frontend-infrastructure`) — router, plugins, API client, zod
  form schemas (`satisfies z.ZodType<GeneratedType>`), consumption of generated types.

## Generated types — never hand-edit

`fe/src/types/generated/` (TS types + `validation.ts`) come from `rdt generate-ts`,
generated from the Rust structs (`#[derive(TS)]` in `crates/core`, `export_bindings`
tests in `crates/api`). A PreToolUse hook (`protect_generated.cjs`) blocks edits to
that directory. Change the Rust structs and regenerate instead.

## i18n

`fe/translations/{en,de}.json`, loaded by `fe/src/composables/useTranslations.ts`
(dynamic import — not vue-i18n). Keep EN/DE keys in parity; the `i18n-translator`
agent owns these files. Components/stores only reference keys.

## Scoped checks (in the Docker `node` container)

```bash
docker compose exec -T node npx vue-tsc --noEmit
docker compose exec -T node npx eslint <paths>
docker compose exec -T node npm run test -- <file>
```
