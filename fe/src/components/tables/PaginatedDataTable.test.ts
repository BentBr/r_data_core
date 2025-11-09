import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import PaginatedDataTable from './PaginatedDataTable.vue'

describe('PaginatedDataTable', () => {
    it('renders total counter in the footer', async () => {
        const wrapper = mount(PaginatedDataTable, {
            props: {
                items: [{ uuid: '1', name: 'A' }, { uuid: '2', name: 'B' }],
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
})


