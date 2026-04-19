#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordResetToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: OffsetDateTime,
    pub created_at: OffsetDateTime,
    pub used_at: Option<OffsetDateTime>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::OffsetDateTime;

    #[test]
    fn password_reset_token_serializes() {
        let token = PasswordResetToken {
            id: Uuid::nil(),
            user_id: Uuid::nil(),
            token_hash: "abc123".to_string(),
            expires_at: OffsetDateTime::UNIX_EPOCH,
            created_at: OffsetDateTime::UNIX_EPOCH,
            used_at: None,
        };
        let json = serde_json::to_string(&token).unwrap();
        assert!(json.contains("abc123"));
        assert!(json.contains("\"used_at\":null"));
    }

    #[test]
    fn password_reset_token_with_used_at() {
        let token = PasswordResetToken {
            id: Uuid::nil(),
            user_id: Uuid::nil(),
            token_hash: "hash".to_string(),
            expires_at: OffsetDateTime::UNIX_EPOCH,
            created_at: OffsetDateTime::UNIX_EPOCH,
            used_at: Some(OffsetDateTime::UNIX_EPOCH),
        };
        let json = serde_json::to_string(&token).unwrap();
        assert!(!json.contains("\"used_at\":null"));
    }
}
