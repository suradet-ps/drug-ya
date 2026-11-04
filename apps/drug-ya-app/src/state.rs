//! Shared backend state for the Tauri process.

use std::sync::atomic::AtomicU64;
use std::time::Instant;

use drug_ya_config::ConfigStore;
use drug_ya_core::SessionUser;
use drug_ya_hosxp::HosxpClient;
use tokio::sync::{Mutex, RwLock};

/// Consecutive failed login attempts and the current lockout deadline.
#[derive(Debug, Default)]
pub struct LoginThrottle {
    /// Number of consecutive failed authentication attempts.
    pub failed_attempts: u32,
    /// When the current lockout ends, if the threshold was reached.
    pub locked_until: Option<Instant>,
}

/// Backend state managed by Tauri. Everything patient-scoped is memory-only.
pub struct AppState {
    /// Encrypted connection + plain review settings.
    pub store: ConfigStore,
    /// Lazily-created HOSxP client for the saved site.
    pub client: RwLock<Option<HosxpClient>>,
    /// Cached connection health.
    pub health: RwLock<crate::commands::ConnectionHealth>,
    /// Signed-in officer (memory only).
    pub session: RwLock<Option<SessionUser>>,
    /// Bumped when the session is cleared or the connection changes.
    pub session_epoch: AtomicU64,
    /// Login throttle.
    pub login_throttle: Mutex<LoginThrottle>,
}

impl AppState {
    /// Open the default store, falling back to an in-memory vault when the
    /// OS keychain is unavailable (headless test environments).
    pub fn new() -> Self {
        let store = match ConfigStore::open() {
            Ok(store) => store,
            Err(error) => {
                tracing::warn!("keychain unavailable, using an in-memory vault: {error}");
                ConfigStore::with_vault(
                    Box::new(crate::commands::EphemeralVault::default()),
                    std::path::PathBuf::new(),
                )
            }
        };
        Self {
            store,
            client: RwLock::new(None),
            health: RwLock::new(crate::commands::ConnectionHealth::Unconfigured),
            session: RwLock::new(None),
            session_epoch: AtomicU64::new(0),
            login_throttle: Mutex::new(LoginThrottle::default()),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
