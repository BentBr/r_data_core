import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import PaginatedDataTable from './PaginatedDataTable.vue'

describe('PaginatedDataTable', () => {
    it('renders total counter in the footer', async () => {
        const wrapper = mount(PaginatedDataTable, {
            props: {
                items: [
                    { uuid: '1', name: 'A' },
                    { uuid: '2', name: 'B' },
                ],
                headers: [{ title: 'Name', key: 'name' }],
                loading: false,
                error: '',
                currentPage: 1,
                itemsPerPage: 10,
                totalItems: 42,
                totalPages: 5,
            },
        })

        const text = wrapper.text().toLowerCase()
        expect(text).toContain('total')
        expect(text).toContain('42')
    })

    it('emits update:sort event when sort options change', async () => {
        const wrapper = mount(PaginatedDataTable, {
            props: {
                items: [
                    { uuid: '1', name: 'A' },
                    { uuid: '2', name: 'B' },
                ],
                headers: [{ title: 'Name', key: 'name', sortable: true }],
                loading: false,
                error: '',
                currentPage: 1,
                itemsPerPage: 10,
                totalItems: 2,
                totalPages: 1,
            },
        })

        // Get the v-data-table-server component
        const dataTable = wrapper.findComponent({ name: 'VDataTableServer' })
        expect(dataTable.exists()).toBe(true)

        // Simulate sort change by calling handleOptionsUpdate directly
        const vm = wrapper.vm as any
        vm.handleOptionsUpdate({
            page: 1,
            itemsPerPage: 10,
            sortBy: [{ key: 'name', order: 'asc' }],
        })

        await wrapper.vm.$nextTick()

        // Check that update:sort event was emitted
        const sortEvents = wrapper.emitted('update:sort')
        expect(sortEvents).toBeDefined()
        expect(sortEvents?.length).toBeGreaterThan(0)
        // The last event should be the sort change
        const lastEvent = sortEvents?.[sortEvents.length - 1]
        expect(lastEvent).toEqual(['name', 'asc'])
    })

    it('emits null sort values when sort is cleared', async () => {
        const wrapper = mount(PaginatedDataTable, {
            props: {
                items: [
                    { uuid: '1', name: 'A' },
                    { uuid: '2', name: 'B' },
                ],
                headers: [{ title: 'Name', key: 'name', sortable: true }],
                loading: false,
                error: '',
                currentPage: 1,
                itemsPerPage: 10,
                totalItems: 2,
                totalPages: 1,
            },
        })

        const vm = wrapper.vm as any
        vm.handleOptionsUpdate({
            page: 1,
            itemsPerPage: 10,
            sortBy: [],
        })

        await wrapper.vm.$nextTick()

        const sortEvents = wrapper.emitted('update:sort')
        expect(sortEvents).toBeDefined()
        expect(sortEvents?.length).toBeGreaterThan(0)
        expect(sortEvents?.[sortEvents.length - 1]).toEqual([null, null])
    })

    it('emits update:page event when page changes', async () => {
        const wrapper = mount(PaginatedDataTable, {
            props: {
                items: [
                    { uuid: '1', name: 'A' },
                    { uuid: '2', name: 'B' },
                ],
                headers: [{ title: 'Name', key: 'name' }],
                loading: false,
                error: '',
                currentPage: 1,
                itemsPerPage: 10,
                totalItems: 20,
                totalPages: 2,
            },
        })

        const vm = wrapper.vm as any
        vm.handleOptionsUpdate({
            page: 2,
            itemsPerPage: 10,
            sortBy: [],
        })

        await wrapper.vm.$nextTick()

        const pageEvents = wrapper.emitted('update:page')
        expect(pageEvents).toBeDefined()
        expect(pageEvents?.length).toBeGreaterThan(0)
        expect(pageEvents?.[0]).toEqual([2])
    })

    it('emits update:items-per-page event when items per page changes', async () => {
        const wrapper = mount(PaginatedDataTable, {
            props: {
                items: [
                    { uuid: '1', name: 'A' },
                    { uuid: '2', name: 'B' },
                ],
                headers: [{ title: 'Name', key: 'name' }],
                loading: false,
                error: '',
                currentPage: 1,
                itemsPerPage: 10,
                totalItems: 20,
                totalPages: 2,
            },
        })

        const vm = wrapper.vm as any
        vm.handleOptionsUpdate({
            page: 1,
            itemsPerPage: 25,
            sortBy: [],
        })

        await wrapper.vm.$nextTick()

        const itemsPerPageEvents = wrapper.emitted('update:items-per-page')
        expect(itemsPerPageEvents).toBeDefined()
        expect(itemsPerPageEvents?.length).toBeGreaterThan(0)
        expect(itemsPerPageEvents?.[0]).toEqual([25])
    })
})
