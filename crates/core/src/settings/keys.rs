#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

/// System setting keys for identifying different configuration settings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemSettingKey {
    /// Entity versioning configuration
    EntityVersioning,
}

impl SystemSettingKey {
    /// Cache key prefix for all system settings
    pub const CACHE_PREFIX: &'static str = "settings:";

    /// Get the string representation of the setting key
    ///
    /// # Returns
    /// The string key used in the database
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::EntityVersioning => "entity_versioning",
        }
    }

    /// Get the cache key for this setting
    ///
    /// # Returns
    /// A formatted cache key string
    #[must_use]
    pub fn cache_key(&self) -> String {
        format!("{}{}", Self::CACHE_PREFIX, self.as_str())
    }
}
