use log::info;
use sqlx::{query, PgPool};

use crate::error::{Error, Result};

/// Create notifications table
pub async fn create_notifications_table(pool: &PgPool) -> Result<()> {
    info!("Creating notifications table...");

    // First create notification status enum
    query(
        r#"
        DO $$
        BEGIN
            IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'notification_status') THEN
                CREATE TYPE notification_status AS ENUM (
                    'pending', 'sending', 'sent', 'read', 'failed', 'cancelled'
                );
            END IF;
        END
        $$;
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(e))?;

    // Then create notification type enum
    query(
        r#"
        DO $$
        BEGIN
            IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'notification_type') THEN
                CREATE TYPE notification_type AS ENUM (
                    'email', 'in_app', 'sms', 'push'
                );
            END IF;
        END
        $$;
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(e))?;

    // Create priority enum
    query(
        r#"
        DO $$
        BEGIN
            IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'notification_priority') THEN
                CREATE TYPE notification_priority AS ENUM (
                    'low', 'normal', 'high', 'urgent'
                );
            END IF;
        END
        $$;
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(e))?;

    // Now create the notifications table
    query(
        r#"
        CREATE TABLE IF NOT EXISTS notifications (
            uuid UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
            notification_type notification_type NOT NULL,
            status notification_status NOT NULL DEFAULT 'pending',
            subject VARCHAR(255) NOT NULL,
            body TEXT NOT NULL,
            recipient_uuid UUID,
            recipient_email VARCHAR(255),
            recipient_phone VARCHAR(50),
            related_entity_uuid VARCHAR(100),
            action_url TEXT,
            priority notification_priority NOT NULL DEFAULT 'normal',
            scheduled_for TIMESTAMPTZ,
            sent_at TIMESTAMPTZ,
            read_at TIMESTAMPTZ,
            retry_count INTEGER NOT NULL DEFAULT 0,
            error_message TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            additional_data JSONB
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(e))?;

    // Create indices
    query(
        "CREATE INDEX IF NOT EXISTS idx_notifications_recipient_uuid ON notifications(recipient_uuid)",
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(e))?;

    query("CREATE INDEX IF NOT EXISTS idx_notifications_status ON notifications(status)")
        .execute(pool)
        .await
        .map_err(|e| Error::Database(e))?;

    Ok(())
}

/// Create workflow definitions table
pub async fn create_workflow_definitions_table(pool: &PgPool) -> Result<()> {
    info!("Creating workflow definitions table...");

    // Create workflow_definitions table
    query(
        r#"
        CREATE TABLE IF NOT EXISTS workflow_definitions (
            uuid UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
            name VARCHAR(255) NOT NULL,
            description TEXT,
            entity_type VARCHAR(100) NOT NULL,
            version INTEGER NOT NULL,
            active BOOLEAN NOT NULL DEFAULT TRUE,
            definition JSONB NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(name, version)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(e))?;

    Ok(())
}

/// Create workflow-related tables
pub async fn create_workflows_table(pool: &PgPool) -> Result<()> {
    info!("Creating workflow tables...");

    // Create workflow status enum
    query(
        r#"
        DO $$
        BEGIN
            IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'workflow_status') THEN
                CREATE TYPE workflow_status AS ENUM (
                    'pending', 'in_progress', 'completed', 'rejected', 'cancelled'
                );
            END IF;
        END
        $$;
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(e))?;

    // Create task status enum
    query(
        r#"
        DO $$
        BEGIN
            IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'task_status') THEN
                CREATE TYPE task_status AS ENUM (
                    'pending', 'in_progress', 'completed', 'skipped', 'failed'
                );
            END IF;
        END
        $$;
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(e))?;

    // Create workflow_instances table
    query(
        r#"
        CREATE TABLE IF NOT EXISTS workflow_instances (
            uuid UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
            workflow_definition_uuid UUID NOT NULL REFERENCES workflow_definitions(uuid),
            entity_uuid VARCHAR(100) NOT NULL,
            entity_type VARCHAR(100) NOT NULL,
            state JSONB NOT NULL,
            status workflow_status NOT NULL,
            created_by UUID,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(e))?;

    // Create workflow_tasks table
    query(
        r#"
        CREATE TABLE IF NOT EXISTS workflow_tasks (
            uuid UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
            workflow_instance_uuid UUID NOT NULL REFERENCES workflow_instances(uuid) ON DELETE CASCADE,
            task_name VARCHAR(255) NOT NULL,
            task_definition JSONB NOT NULL,
            status task_status NOT NULL,
            result JSONB,
            assigned_to UUID,
            due_date TIMESTAMPTZ,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            completed_at TIMESTAMPTZ
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(e))?;

    // Create workflow_history table
    query(
        r#"
        CREATE TABLE IF NOT EXISTS workflow_history (
            uuid UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
            workflow_instance_uuid UUID NOT NULL REFERENCES workflow_instances(uuid) ON DELETE CASCADE,
            event_type VARCHAR(100) NOT NULL,
            event_data JSONB NOT NULL,
            performed_by UUID,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(e))?;

    Ok(())
}
