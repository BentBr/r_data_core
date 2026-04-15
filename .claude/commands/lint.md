# Run Frontend Linting

Run ESLint and Prettier for frontend code.

## Commands

### Admin Frontend (fe/)
```bash
rdt lint
```

## Instructions

1. Identify which frontend needs linting based on context
2. Run the appropriate lint command
3. If linting fails, analyze errors and suggest fixes
4. Common issues: unused imports, formatting, TypeScript errors

## Manual commands (if rdt unavailable)

```bash
# Admin frontend
docker compose exec node npm run lint
docker compose exec node npm run lint:fix

# Or directly in fe/ directory
cd fe && npm run lint
```
