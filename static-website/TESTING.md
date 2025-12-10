# Testing Guide

## Overview

The static website has comprehensive test coverage using Vitest and Vue Test Utils.

## Test Stats

- **51 tests** across 9 test files
- **100% passing** rate
- Test execution time: ~2 seconds

## Test Structure

```
src/
├── components/
│   └── __tests__/
│       ├── Header.spec.ts
│       └── Footer.spec.ts
├── components/common/
│   └── __tests__/
│       ├── ThemeToggle.spec.ts
│       └── LanguageSwitch.spec.ts
├── composables/
│   └── __tests__/
│       └── useTranslations.spec.ts
├── router/
│   └── __tests__/
│       └── index.spec.ts
├── env-check.spec.ts
└── test/
    ├── setup.ts
    └── mocks/
        └── styleMock.ts

scripts/
└── __tests__/
    ├── generate-images.spec.js
    └── generate-sitemap.spec.js
```

## Running Tests

```bash
# Run all tests
npm run test

# Watch mode (re-run on changes)
npm run test:watch

# Coverage report
npm run test:coverage

# Interactive UI
npm run test:ui
```

## Test Coverage

### Components (14 tests)
- ✅ Header component rendering and functionality
- ✅ Footer component rendering and links
- ✅ ThemeToggle dark/light mode switching
- ✅ LanguageSwitch language toggle

### Composables (9 tests)
- ✅ useTranslations language initialization
- ✅ useTranslations language switching
- ✅ useTranslations key translation
- ✅ useTranslations nested keys
- ✅ useTranslations missing keys handling
- ✅ useTranslations get function
- ✅ useTranslations locale persistence

### Environment & Config (7 tests)
- ✅ env-check URL handling
- ✅ env-check property existence
- ✅ env-check default values
- ✅ env-check environment detection

### Router (4 tests)
- ✅ Router initialization
- ✅ Language-based routing
- ✅ Root redirect to language path
- ✅ Scroll behavior

### Scripts (17 tests)
- ✅ generate-images structure
- ✅ generate-images favicon sizes
- ✅ generate-images responsive widths
- ✅ generate-images OG dimensions
- ✅ generate-sitemap page definitions
- ✅ generate-sitemap language support
- ✅ generate-sitemap noindex exclusion
- ✅ generate-sitemap file structure

## Test Configuration

### Vitest Config (`vitest.config.ts`)
- Environment: happy-dom
- Globals: enabled
- CSS: disabled for performance
- Coverage: v8 provider
- Test setup: `src/test/setup.ts`

### Test Setup (`src/test/setup.ts`)
- Vuetify instance configured
- window.matchMedia mocked
- IntersectionObserver mocked
- Minimal component registration

## CI/CD Integration

Tests run automatically on every push and pull request via GitHub Actions:

```yaml
static-website-test:
  - Install dependencies
  - Run npm test
  - Report results
```

Part of the `ci-gate` that blocks merging if tests fail.

## Writing New Tests

### Component Test Example

```typescript
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createMemoryHistory } from 'vue-router'
import MyComponent from '../MyComponent.vue'

const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/', component: { template: '<div>Home</div>' } }],
})

describe('MyComponent', () => {
    it('should render', () => {
        const wrapper = mount(MyComponent, {
            global: {
                plugins: [router],
            },
        })
        expect(wrapper.exists()).toBe(true)
    })
})
```

### Composable Test Example

```typescript
import { describe, it, expect } from 'vitest'
import { useMyComposable } from '../useMyComposable'

describe('useMyComposable', () => {
    it('should return expected value', () => {
        const { value } = useMyComposable()
        expect(value).toBeDefined()
    })
})
```

### Script Test Example

```javascript
import { describe, it, expect } from 'vitest'

describe('my-script', () => {
    it('should validate configuration', () => {
        const config = { key: 'value' }
        expect(config).toHaveProperty('key')
    })
})
```

## Best Practices

1. **Isolation**: Each test should be independent
2. **Descriptive names**: Use clear, descriptive test names
3. **AAA pattern**: Arrange, Act, Assert
4. **Mock external dependencies**: API calls, localStorage, etc.
5. **Test behavior, not implementation**: Focus on what, not how
6. **Coverage**: Aim for high coverage, but prioritize critical paths

## Common Issues & Solutions

### CSS Import Errors
**Problem**: `Unknown file extension ".css"`  
**Solution**: CSS processing disabled in vitest.config.ts

### Router Warnings
**Problem**: `No match found for location`  
**Solution**: Create minimal test routes with correct paths

### Vuetify Components Not Found
**Problem**: Components not registered in tests  
**Solution**: Import and register in `src/test/setup.ts`

### Async Tests Failing
**Problem**: Tests complete before async operations  
**Solution**: Use `await` with `nextTick()` or `flushPromises()`

## Debugging Tests

```bash
# Run specific test file
npx vitest run src/components/__tests__/Header.spec.ts

# Run tests matching pattern
npx vitest run --grep="Header"

# Run in debug mode
npx vitest --inspect-brk

# UI mode for visual debugging
npm run test:ui
```

## Future Improvements

- [ ] Add E2E tests with Playwright
- [ ] Increase coverage to 90%+
- [ ] Add visual regression tests
- [ ] Add performance benchmarks
- [ ] Add accessibility (a11y) tests

