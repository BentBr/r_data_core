import { describe, it, expect } from 'vitest'
import type { PaginationQuery } from '@/types/generated/PaginationQuery'
import type { SortingQuery } from '@/types/generated/SortingQuery'
import { buildListQueryString } from './query'

const allNullPagination: PaginationQuery = {
    page: null,
    per_page: null,
    limit: null,
    offset: null,
}

describe('buildListQueryString', () => {
    it('returns empty string when every field is null and no sorting is passed', () => {
        expect(buildListQueryString(allNullPagination)).toBe('')
    })

    it('returns empty string when sorting is undefined and pagination is empty', () => {
        expect(buildListQueryString(allNullPagination, undefined)).toBe('')
    })

    it('emits page + per_page when both are set', () => {
        const pagination: PaginationQuery = {
            page: 2,
            per_page: 25,
            limit: null,
            offset: null,
        }
        expect(buildListQueryString(pagination)).toBe('?page=2&per_page=25')
    })

    it('emits limit + offset when both are set', () => {
        const pagination: PaginationQuery = {
            page: null,
            per_page: null,
            limit: 10,
            offset: 40,
        }
        expect(buildListQueryString(pagination)).toBe('?limit=10&offset=40')
    })

    it('preserves zero values (does not treat 0 as absent)', () => {
        const pagination: PaginationQuery = {
            page: null,
            per_page: null,
            limit: 20,
            offset: 0,
        }
        expect(buildListQueryString(pagination)).toBe('?limit=20&offset=0')
    })

    it('appends sort_by + sort_order when both are set', () => {
        const pagination: PaginationQuery = {
            page: 1,
            per_page: 10,
            limit: null,
            offset: null,
        }
        const sorting: SortingQuery = { sort_by: 'name', sort_order: 'asc' }
        expect(buildListQueryString(pagination, sorting)).toBe(
            '?page=1&per_page=10&sort_by=name&sort_order=asc'
        )
    })

    it('skips sorting when sort_by is null even if sort_order is set', () => {
        const sorting: SortingQuery = { sort_by: null, sort_order: 'desc' }
        expect(buildListQueryString(allNullPagination, sorting)).toBe('')
    })

    it('skips sorting when sort_order is null even if sort_by is set', () => {
        const sorting: SortingQuery = { sort_by: 'name', sort_order: null }
        expect(buildListQueryString(allNullPagination, sorting)).toBe('')
    })

    it('ignores sorting when passed null explicitly', () => {
        const pagination: PaginationQuery = {
            page: 1,
            per_page: 10,
            limit: null,
            offset: null,
        }
        expect(buildListQueryString(pagination, null)).toBe('?page=1&per_page=10')
    })

    it('combines page/per_page with sort params correctly', () => {
        const pagination: PaginationQuery = {
            page: 3,
            per_page: 50,
            limit: null,
            offset: null,
        }
        const sorting: SortingQuery = { sort_by: 'created_at', sort_order: 'desc' }
        expect(buildListQueryString(pagination, sorting)).toBe(
            '?page=3&per_page=50&sort_by=created_at&sort_order=desc'
        )
    })
})
