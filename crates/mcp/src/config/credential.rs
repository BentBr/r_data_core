#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! A secret that does not print itself.

/// A secret read from the environment.
///
/// `Debug` prints a placeholder. Nothing prints `Config` today, but the one
/// time someone adds `{config:?}` to a startup log is the one time it would
/// matter, and this makes that line safe before it is written.
#[derive(Clone, PartialEq, Eq)]
pub struct Credential(String);

impl Credential {
    #[must_use]
    pub fn new(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    /// The secret itself, for the one place that sends it.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Credential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Credential(\"***\")")
    }
}
