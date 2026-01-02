#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};
use std::fmt;

/// License type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LicenseType {
    /// Community edition
    #[serde(rename = "community")]
    Community,
    /// Education edition
    #[serde(rename = "education")]
    Education,
    /// Company tier I
    #[serde(rename = "company I")]
    CompanyI,
    /// Company tier II
    #[serde(rename = "company II")]
    CompanyII,
    /// Company tier III
    #[serde(rename = "company III")]
    CompanyIII,
    /// Enterprise edition
    #[serde(rename = "Enterprise")]
    Enterprise,
    /// Society tier I
    #[serde(rename = "society I")]
    SocietyI,
    /// Society tier II
    #[serde(rename = "society II")]
    SocietyII,
    /// Society tier III
    #[serde(rename = "society III")]
    SocietyIII,
}

impl LicenseType {
    #[must_use]
    pub const fn all_variants() -> &'static [&'static str] {
        &[
            "community",
            "education",
            "company I",
            "company II",
            "company III",
            "Enterprise",
            "society I",
            "society II",
            "society III",
        ]
    }
}

impl fmt::Display for LicenseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Community => write!(f, "community"),
            Self::Education => write!(f, "education"),
            Self::CompanyI => write!(f, "company I"),
            Self::CompanyII => write!(f, "company II"),
            Self::CompanyIII => write!(f, "company III"),
            Self::Enterprise => write!(f, "Enterprise"),
            Self::SocietyI => write!(f, "society I"),
            Self::SocietyII => write!(f, "society II"),
            Self::SocietyIII => write!(f, "society III"),
        }
    }
}

impl std::str::FromStr for LicenseType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "community" => Ok(Self::Community),
            "education" => Ok(Self::Education),
            "company I" => Ok(Self::CompanyI),
            "company II" => Ok(Self::CompanyII),
            "company III" => Ok(Self::CompanyIII),
            "Enterprise" => Ok(Self::Enterprise),
            "society I" => Ok(Self::SocietyI),
            "society II" => Ok(Self::SocietyII),
            "society III" => Ok(Self::SocietyIII),
            _ => Err(format!(
                "Invalid license type: {}",
                Self::all_variants().join(", ")
            )),
        }
    }
}

/// License claims structure for JWT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseClaims {
    /// License version
    pub version: String,
    /// Company name
    pub company: String,
    /// License type
    #[serde(rename = "license_type")]
    pub license_type: LicenseType,
    /// Issue date (ISO 8601)
    #[serde(rename = "issued_at")]
    pub issued_at: String,
    /// License ID (UUID v7)
    #[serde(rename = "license_id")]
    pub license_id: String,
    /// Expiration timestamp (Unix epoch)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<i64>,
}
