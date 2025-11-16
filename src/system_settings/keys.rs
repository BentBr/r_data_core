#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemSettingKey {
    EntityVersioning,
}

impl SystemSettingKey {
    pub const CACHE_PREFIX: &'static str = "settings:";

    pub fn as_str(&self) -> &'static str {
        match self {
            SystemSettingKey::EntityVersioning => "entity_versioning",
        }
    }

    pub fn cache_key(&self) -> String {
        format!("{}{}", Self::CACHE_PREFIX, self.as_str())
    }
}
