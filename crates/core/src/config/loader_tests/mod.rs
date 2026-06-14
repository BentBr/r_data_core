#![allow(clippy::unwrap_used)]
#![allow(clippy::redundant_closure_for_method_calls)]

//! Tests for `config::loader` — env-var parsing for all config loaders.
//!
//! **Env-var safety note:** The loader functions all call `dotenv().ok()` which
//! re-loads the project `.env` file on every call.  This means any var in
//! `.env` cannot be tested as "absent" — removing it from the process env is
//! undone by the next `dotenv()` call inside the loader.
//!
//! Strategy:
//! - Tests that need a var to have a specific value: save, override, test,
//!   then restore via `EnvGuard` (RAII).
//! - Tests for "missing required var" errors: override the var with an empty
//!   string (`""`) so the `env::var` call returns a value but the `map_err`
//!   path for truly-absent vars is tested via value-based error paths (zero,
//!   negative, non-numeric, etc.).
//! - All env-mutating tests hold `ENV_MUTEX` to prevent cross-test races.

use std::collections::HashMap;
use std::env;
use std::sync::Mutex;

use super::{load_cache_config, load_license_config, load_worker_config};

mod app_config;
mod cache_config;
mod license_config;
mod maintenance_config;
mod outbox_config;
mod worker_config;

// A process-wide mutex that all env-mutating tests must hold.
pub(super) static ENV_MUTEX: Mutex<()> = Mutex::new(());

pub(super) struct EnvGuard {
    saved: HashMap<String, Option<String>>,
}

impl EnvGuard {
    /// Save current values for each key in `overrides`, then apply the
    /// override.  `Some(val)` sets the var; `None` removes it.
    pub(super) fn new(overrides: &[(&str, Option<&str>)]) -> Self {
        let mut saved = HashMap::new();
        for &(k, v) in overrides {
            saved.insert(k.to_string(), env::var(k).ok());
            match v {
                Some(val) => unsafe { env::set_var(k, val) },
                None => unsafe { env::remove_var(k) },
            }
        }
        Self { saved }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (k, v) in &self.saved {
            match v {
                Some(val) => unsafe { env::set_var(k, val) },
                None => unsafe { env::remove_var(k) },
            }
        }
    }
}

pub(super) fn minimal_worker_overrides<'a>() -> Vec<(&'a str, Option<&'a str>)> {
    vec![
        ("JOB_QUEUE_UPDATE_INTERVAL", Some("5")),
        ("WORKER_DATABASE_URL", Some("postgres://localhost/worker")),
        ("REDIS_URL", Some("redis://localhost:6379")),
    ]
}
