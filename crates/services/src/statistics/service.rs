#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

use r_data_core_core::config::LicenseConfig;
use r_data_core_license::LicenseClaims;
use r_data_core_persistence::{
    EntityCount, EntityDefinitionsStats, StatisticsRepository, StatisticsRepositoryTrait,
};
use uuid::Uuid;

/// Statistics payload to send to the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticsPayload {
    /// Unique submission ID (UUID v7 timestamp-based)
    pub submission_id: Uuid,
    /// Entity definitions count and names
    pub entity_definitions: EntityDefinitionsStats,
    /// Entities count per definition
    pub entities_per_definition: Vec<EntityCount>,
    /// Total users count
    pub users_count: i64,
    /// Total roles count
    pub roles_count: i64,
    /// Total API keys count
    pub api_keys_count: i64,
    /// Total workflows count
    pub workflows_count: i64,
    /// Total workflow logs count
    pub workflow_logs_count: i64,
    /// Admin URI
    pub admin_uri: String,
    /// CORS origins
    pub cors_origins: Vec<String>,
    /// License key ID (always present - from license or deterministic hash)
    pub license_key_id: String,
    /// Submission timestamp (ISO 8601)
    pub submitted_at: String,
}

/// Service for collecting and sending statistics
pub struct StatisticsService {
    /// License configuration
    config: LicenseConfig,
    /// HTTP client for API calls
    client: Client,
    /// Statistics repository
    repository: Arc<StatisticsRepository>,
    /// Database URL (used as fallback seed for generating instance key when unlicensed)
    database_url: Option<String>,
}

impl StatisticsService {
    /// Create a new statistics service
    ///
    /// # Arguments
    /// * `config` - License configuration
    /// * `repository` - Statistics repository
    #[must_use]
    pub fn new(config: LicenseConfig, repository: Arc<StatisticsRepository>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        Self {
            config,
            client,
            repository,
            database_url: None,
        }
    }

    /// Create a new statistics service with database URL for unlicensed instance key generation
    ///
    /// # Arguments
    /// * `config` - License configuration
    /// * `repository` - Statistics repository
    /// * `database_url` - Database URL (used as seed for generating instance key when unlicensed)
    #[must_use]
    pub fn with_database_url(
        config: LicenseConfig,
        repository: Arc<StatisticsRepository>,
        database_url: String,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        Self {
            config,
            client,
            repository,
            database_url: Some(database_url),
        }
    }

    /// Collect and send statistics
    ///
    /// This method silently fails - errors are only printed to stdout, not logged
    pub async fn collect_and_send(&self, admin_uri: &str, cors_origins: &[String]) {
        // Collect statistics
        let payload = match self.collect_statistics(admin_uri, cors_origins).await {
            Ok(p) => p,
            Err(e) => {
                println!("Statistics collection failed: {e}");
                return;
            }
        };

        // Send to API (silent failure)
        if let Err(e) = self.send_statistics(&payload).await {
            println!("Statistics submission failed: {e}");
        }
    }

    /// Collect statistics from the database
    ///
    /// # Errors
    /// Returns an error if database queries fail
    async fn collect_statistics(
        &self,
        admin_uri: &str,
        cors_origins: &[String],
    ) -> Result<StatisticsPayload, Box<dyn std::error::Error + Send + Sync>> {
        use sha2::{Digest, Sha256};
        use time::OffsetDateTime;
        use uuid::Uuid;

        // Generate unique submission ID (UUID v7 for timestamp ordering)
        let submission_id = Uuid::now_v7();

        // Get current timestamp (ISO 8601)
        let submitted_at = OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|e| format!("Failed to format timestamp: {e}"))?;

        // Get license key ID, or generate a deterministic one for unlicensed instances
        let license_key_id = self.get_license_key_id().ok().unwrap_or_else(|| {
            // For unlicensed instances, generate a deterministic UUID from the database URL
            // This ensures the same instance always reports with the same key
            let seed = self
                .database_url
                .as_ref()
                .map_or_else(|| admin_uri.to_string(), Clone::clone);

            let mut hasher = Sha256::new();
            hasher.update(seed.as_bytes());
            let hash = hasher.finalize();

            // Create a deterministic UUID from the hash (UUID v5 style)
            let mut uuid_bytes = [0u8; 16];
            uuid_bytes.copy_from_slice(&hash[..16]);
            // Set version to 5 (SHA-1 based, we use SHA-256 but the same idea)
            uuid_bytes[6] = (uuid_bytes[6] & 0x0f) | 0x50;
            // Set variant to RFC 4122
            uuid_bytes[8] = (uuid_bytes[8] & 0x3f) | 0x80;

            Uuid::from_bytes(uuid_bytes).to_string()
        });

        // Get entity definitions count and names
        let entity_definitions = self.repository.get_entity_definitions_stats().await?;

        // Get entities per definition
        let entities_per_definition = self.repository.get_entities_per_definition().await?;

        // Get users count
        let users_count = self.repository.get_users_count().await?;

        // Get roles count
        let roles_count = self.repository.get_roles_count().await?;

        // Get API keys count
        let api_keys_count = self.repository.get_api_keys_count().await?;

        // Get workflows count
        let workflows_count = self.repository.get_workflows_count().await?;

        // Get workflow logs count
        let workflow_logs_count = self.repository.get_workflow_logs_count().await?;

        Ok(StatisticsPayload {
            submission_id,
            entity_definitions,
            entities_per_definition,
            users_count,
            roles_count,
            api_keys_count,
            workflows_count,
            workflow_logs_count,
            admin_uri: admin_uri.to_string(),
            cors_origins: cors_origins.to_vec(),
            license_key_id,
            submitted_at,
        })
    }

    /// Get license key ID from license key
    fn get_license_key_id(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Decode JWT to get license_id
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;

        let license_key = self
            .config
            .license_key
            .as_ref()
            .ok_or("No license key configured")?;

        let parts: Vec<&str> = license_key.split('.').collect();
        if parts.len() != 3 {
            return Err("Invalid JWT format".into());
        }

        let payload = parts[1];
        let decoded = URL_SAFE_NO_PAD.decode(payload)?;
        let claims: LicenseClaims = serde_json::from_slice(&decoded)?;

        Ok(claims.license_id)
    }

    /// Send statistics to the API
    ///
    /// # Errors
    /// Returns an error if the API call fails
    async fn send_statistics(
        &self,
        payload: &StatisticsPayload,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .post(&self.config.statistics_url)
            .json(payload)
            .send()
            .await?;
        Ok(())
    }
}
