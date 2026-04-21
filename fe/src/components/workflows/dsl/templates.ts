import type { WorkflowTemplate } from './contracts'

export function createWorkflowTemplates(): WorkflowTemplate[] {
    return [
        {
            id: 'api_ingest',
            steps: [
                {
                    from: {
                        type: 'format',
                        source: {
                            source_type: 'api',
                            config: {},
                            auth: { type: 'none' },
                        },
                        format: {
                            format_type: 'json',
                            options: {},
                        },
                        mapping: {},
                    },
                    transform: { type: 'none' },
                    to: {
                        type: 'entity',
                        entity_definition: '',
                        path: '/',
                        mode: 'create',
                        mapping: {},
                    },
                },
            ],
        },
        {
            id: 'remote_pull',
            steps: [
                {
                    from: {
                        type: 'format',
                        source: {
                            source_type: 'uri',
                            config: { uri: '' },
                            auth: { type: 'none' },
                        },
                        format: {
                            format_type: 'json',
                            options: {},
                        },
                        mapping: {},
                    },
                    transform: { type: 'none' },
                    to: {
                        type: 'entity',
                        entity_definition: '',
                        path: '/',
                        mode: 'create',
                        mapping: {},
                    },
                },
            ],
        },
        {
            id: 'entity_export',
            steps: [
                {
                    from: {
                        type: 'entity',
                        entity_definition: '',
                        mapping: {},
                    },
                    transform: { type: 'none' },
                    to: {
                        type: 'format',
                        output: {
                            mode: 'push',
                            destination: {
                                destination_type: 'uri',
                                config: { uri: '' },
                                auth: { type: 'none' },
                            },
                            method: 'POST',
                        },
                        format: {
                            format_type: 'json',
                            options: {},
                        },
                        mapping: {},
                    },
                },
            ],
        },
        {
            id: 'email_notification',
            steps: [
                {
                    from: {
                        type: 'format',
                        source: {
                            source_type: 'api',
                            config: {},
                            auth: { type: 'none' },
                        },
                        format: {
                            format_type: 'json',
                            options: {},
                        },
                        mapping: {},
                    },
                    transform: {
                        type: 'send_email',
                        template_uuid: '',
                        to: [{ kind: 'field', field: 'email' }],
                        target_status: 'email_status',
                    },
                    to: {
                        type: 'format',
                        output: { mode: 'api' },
                        format: {
                            format_type: 'json',
                            options: {},
                        },
                        mapping: { status: 'email_status' },
                    },
                },
            ],
        },
    ]
}
