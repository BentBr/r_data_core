import type { ValidationViolation } from '@/types/schemas'

// Custom error class for validation errors thrown by api/clients/base.ts.
export class ValidationError extends Error {
    violations: ValidationViolation[]

    constructor(message: string, violations: ValidationViolation[]) {
        super(message)
        this.name = 'ValidationError'
        this.violations = violations
    }
}
