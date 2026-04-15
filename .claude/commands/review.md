# Code Review

Review code changes in the current branch or a specific PR.

## Instructions

When reviewing code for this project, check for:

### Rust Backend
1. **Clippy compliance** - Run `rdt clippy` to catch lint issues
2. **Test coverage** - New features should have tests
3. **Error handling** - Use proper error types, not unwrap() in production code
4. **SQLx queries** - Ensure `.sqlx/` is updated if queries changed
5. **Security** - Check for SQL injection, XSS, command injection
6. **Performance** - Avoid unnecessary clones, use references
7. **Documentation** - Public APIs should be documented

### Frontend (Vue/TypeScript)
1. **TypeScript strict mode** - No any types without justification
2. **Zod validation** - Input validation for API responses
3. **Component structure** - Follow Vue 3 composition API patterns
4. **Pinia stores** - State management follows existing patterns

### General
1. **Commits** - Clear, descriptive messages
2. **Breaking changes** - Document in PR description
3. **Migrations** - Backward compatible when possible

## Commands for review

```bash
# Check git diff
git diff main...HEAD

# Run all checks
rdt test && rdt clippy && rdt lint

# Check specific files changed
git diff --name-only main...HEAD
```
