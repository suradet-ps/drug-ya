//! Connection configuration for a HOSxP site.

use secrecy::SecretString;

/// HOSxP connection settings. The password is a `SecretString` and is never
/// logged or serialized in plaintext.
#[derive(Debug, Clone)]
pub struct HosxpConfig {
    /// Hostname or IP.
    pub host: String,
    /// TCP port (usually 3306).
    pub port: u16,
    /// Database name.
    pub database: String,
    /// Database user - use a `GRANT SELECT`-only account in production.
    pub user: String,
    /// Database password.
    pub password: SecretString,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_password() {
        let config = HosxpConfig {
            host: "db".into(),
            port: 3306,
            database: "hos".into(),
            user: "ro".into(),
            password: SecretString::from("super-secret"),
        };
        let debug = format!("{config:?}");
        assert!(!debug.contains("super-secret"));
    }
}
