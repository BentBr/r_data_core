interface ApiResponse<T> {
    status: 'Success' | 'Error'
    message: string
    data?: T
    meta?: {
        pagination?: {
            total: number
            page: number
            per_page: number
            total_pages: number
            has_previous: boolean
            has_next: boolean
        }
        request_id: string
        timestamp: string
        custom?: Record<string, unknown>
    }
}

class HttpClient {
    private baseURL = import.meta.env.VITE_API_BASE_URL || 'http://localhost:8888'

    async request<T>(endpoint: string, options: RequestInit = {}): Promise<ApiResponse<T>> {
        // TODO: Import auth store when created
        // const authStore = useAuthStore()

        const config: RequestInit = {
            ...options,
            headers: {
                'Content-Type': 'application/json',
                // TODO: Add auth token when auth store is ready
                // ...(authStore.token && {
                //   Authorization: `Bearer ${authStore.token}`
                // }),
                ...options.headers,
            },
        }

        try {
            const response = await fetch(`${this.baseURL}${endpoint}`, config)

            if (!response.ok) {
                if (response.status === 401) {
                    // TODO: Handle logout when auth store is ready
                    // authStore.logout()
                    throw new Error('Authentication required')
                }
                throw new Error(`HTTP ${response.status}`)
            }

            return await response.json()
        } catch (error) {
            console.error('API Error:', error)
            throw error
        }
    }

    get<T>(endpoint: string) {
        return this.request<T>(endpoint, { method: 'GET' })
    }

    post<T>(endpoint: string, data?: unknown) {
        return this.request<T>(endpoint, {
            method: 'POST',
            body: JSON.stringify(data),
        })
    }

    put<T>(endpoint: string, data?: unknown) {
        return this.request<T>(endpoint, {
            method: 'PUT',
            body: JSON.stringify(data),
        })
    }

    delete<T>(endpoint: string) {
        return this.request<T>(endpoint, { method: 'DELETE' })
    }
}

export const httpClient = new HttpClient()
export type { ApiResponse }
