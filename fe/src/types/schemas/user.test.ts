/**
 * Zod schema tests for the user forms — in particular the `status` field that
 * carries an operator unlock through to the backend.
 */
import { describe, it, expect } from 'vitest'
import { UserStatusSchema, UpdateUserRequestSchema, CreateUserRequestSchema } from './user'
import type { UserStatus } from '../generated/UserStatus'

describe('UserStatusSchema', () => {
    const allVariants: UserStatus[] = ['active', 'inactive', 'locked', 'pending_activation']

    it.each(allVariants)('accepts the generated variant %s', variant => {
        expect(UserStatusSchema.parse(variant)).toBe(variant)
    })

    it('covers every variant of the generated union', () => {
        // Guards against the Rust enum gaining a variant the FE silently drops.
        expect(UserStatusSchema.options).toEqual(allVariants)
    })

    it.each(['unlocked', 'ACTIVE', 'pending-activation', '', 'Locked'])('rejects %s', invalid => {
        expect(UserStatusSchema.safeParse(invalid).success).toBe(false)
    })

    it('rejects non-string input', () => {
        expect(UserStatusSchema.safeParse(null).success).toBe(false)
        expect(UserStatusSchema.safeParse(1).success).toBe(false)
    })
})

describe('UpdateUserRequestSchema', () => {
    it('accepts an unlock-only payload', () => {
        const parsed = UpdateUserRequestSchema.parse({ status: 'active' })
        expect(parsed).toEqual({ status: 'active' })
    })

    it('leaves status absent when it is not supplied', () => {
        const parsed = UpdateUserRequestSchema.parse({ first_name: 'Nobody' })
        expect(parsed.status).toBeUndefined()
    })

    it('rejects a status the backend enum does not have', () => {
        expect(UpdateUserRequestSchema.safeParse({ status: 'unlocked' }).success).toBe(false)
    })

    it('still validates the other fields alongside status', () => {
        expect(
            UpdateUserRequestSchema.safeParse({ status: 'active', email: 'not-an-email' }).success
        ).toBe(false)
        expect(
            UpdateUserRequestSchema.safeParse({ status: 'active', password: 'short' }).success
        ).toBe(false)
    })

    it('accepts a full edit payload including status', () => {
        const payload = {
            email: 'someone@example.com',
            password: 'longenoughpassword',
            first_name: 'Some',
            last_name: 'One',
            role_uuids: ['01923e4a-aaaa-7d8e-9f01-234567890abc'],
            is_active: true,
            super_admin: false,
            status: 'locked' as const,
        }
        expect(UpdateUserRequestSchema.parse(payload)).toEqual(payload)
    })
})

describe('CreateUserRequestSchema', () => {
    it('does not accept a status on creation', () => {
        // New users start Active; status is an update-only concern.
        const parsed = CreateUserRequestSchema.parse({
            username: 'someone',
            email: 'someone@example.com',
            password: 'longenoughpassword',
            first_name: 'Some',
            last_name: 'One',
            status: 'locked',
        })
        expect(parsed).not.toHaveProperty('status')
    })
})
