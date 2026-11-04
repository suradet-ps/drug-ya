//! Frontend API layer - the only place the webview talks to the Tauri
//! backend. No SQL, hosts, or credentials live here.

use drug_ya_core::model::{Patient, PregnancyStatus, ReviewResult, RulePackInfo};
use drug_ya_core::query_kind::SearchKind;
use drug_ya_core::rules::{ReviewSettings, RuleConfigView};
use serde::{Deserialize, Serialize};

/// Failure class of a backend command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ApiErrorKind {
    /// No connection settings stored.
    NotConfigured,
    /// HOSxP could not be reached.
    Connection,
    /// The read-only guard rejected a statement.
    Guard,
    /// The query or data was unusable.
    Query,
    /// Not signed in / credentials rejected.
    Auth,
    /// Submitted input is not usable.
    Validation,
}

/// A typed command failure with a Thai message for the user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    /// Failure class.
    pub kind: ApiErrorKind,
    /// Thai message.
    pub message: String,
}

impl ApiError {
    fn from_bridge(error: drug_ya_bridge::BridgeError) -> Self {
        if let drug_ya_bridge::BridgeError::Command(text) = &error
            && let Ok(typed) = serde_json::from_str::<ApiError>(text)
        {
            return typed;
        }
        Self {
            kind: ApiErrorKind::Query,
            message: error.to_string(),
        }
    }
}

/// Connection form payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInput {
    /// Hostname or IP.
    pub host: String,
    /// TCP port.
    pub port: u16,
    /// Database name.
    pub database: String,
    /// Database user.
    pub user: String,
    /// Password (never stored by the frontend).
    pub password: String,
}

/// Non-secret connection summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInfo {
    /// Host.
    pub host: String,
    /// Port.
    pub port: u16,
    /// Database.
    pub database: String,
    /// User.
    pub user: String,
}

/// Login credentials.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginInput {
    /// HOSxP login name.
    pub username: String,
    /// Password.
    pub password: String,
}

/// App status for the login screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    /// Whether a connection is saved.
    pub configured: bool,
    /// Site label.
    pub site_name: String,
    /// Host, when configured.
    pub host: Option<String>,
    /// Database, when configured.
    pub database: Option<String>,
    /// Database user, when configured.
    pub user: Option<String>,
    /// Rule pack version.
    pub rule_pack_version: String,
    /// Whether the pack is signed off.
    pub signed_off: bool,
}

/// Connection health for the top-bar dot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionHealth {
    /// No saved configuration.
    Unconfigured,
    /// Reachable.
    Connected,
    /// Unreachable.
    Disconnected,
}

/// `SELECT 1` result.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTestResult {
    /// Round-trip latency.
    pub latency_ms: u64,
}

/// Settings payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewSettingsResponse {
    /// Current settings.
    pub settings: ReviewSettings,
    /// Pack metadata.
    pub pack: RulePackInfo,
    /// Rule rows.
    pub rules: Vec<RuleConfigView>,
}

/// Sign-off form.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignoffInput {
    /// Pharmacist name.
    pub pharmacist_name: String,
    /// Licence number.
    pub license_no: String,
    /// Note.
    pub note: String,
}

async fn invoke_raw<T>(command: &str, args: impl Serialize) -> Result<T, ApiError>
where
    T: serde::de::DeserializeOwned,
{
    drug_ya_bridge::invoke::<T>(command, args)
        .await
        .map_err(ApiError::from_bridge)
}

async fn call_empty<T>(command: &str) -> Result<T, ApiError>
where
    T: serde::de::DeserializeOwned,
{
    invoke_raw(command, serde_json::json!({})).await
}

/// Reveal the main window.
pub async fn show_main_window() -> Result<(), ApiError> {
    call_empty("show_main_window").await
}

/// Login-screen status.
pub async fn get_app_status() -> Result<AppStatus, ApiError> {
    call_empty("get_app_status").await
}

/// Live connection health.
pub async fn connection_health() -> Result<ConnectionHealth, ApiError> {
    call_empty("connection_health").await
}

/// Verify officer credentials.
pub async fn login(username: &str, password: &str) -> Result<drug_ya_core::SessionUser, ApiError> {
    invoke_raw(
        "login",
        serde_json::json!({
            "input": LoginInput {
                username: username.to_string(),
                password: password.to_string(),
            }
        }),
    )
    .await
}

/// Clear the session.
pub async fn logout() -> Result<(), ApiError> {
    call_empty("logout").await
}

/// Current session.
pub async fn current_session() -> Result<Option<drug_ya_core::SessionUser>, ApiError> {
    call_empty("current_session").await
}

/// Save the encrypted connection and connect.
pub async fn save_connection(config: &ConnectionInput) -> Result<(), ApiError> {
    invoke_raw("save_connection", serde_json::json!({ "config": config })).await
}

/// Saved connection summary.
pub async fn get_connection() -> Result<ConnectionInfo, ApiError> {
    call_empty("get_connection").await
}

/// Test connectivity without saving.
pub async fn test_connection(
    config: Option<&ConnectionInput>,
) -> Result<ConnectionTestResult, ApiError> {
    invoke_raw("test_connection", serde_json::json!({ "config": config })).await
}

/// Review settings + rule catalog.
pub async fn get_review_settings() -> Result<ReviewSettingsResponse, ApiError> {
    call_empty("get_review_settings").await
}

/// Save review settings.
pub async fn save_review_settings(settings: &ReviewSettings) -> Result<(), ApiError> {
    invoke_raw(
        "save_review_settings",
        serde_json::json!({ "settings": settings }),
    )
    .await
}

/// Sign off the current rule pack.
pub async fn sign_off_rule_pack(input: &SignoffInput) -> Result<RulePackInfo, ApiError> {
    invoke_raw("sign_off_rule_pack", serde_json::json!({ "input": input })).await
}

/// Search patients.
pub async fn search_patients(query: &str, kind: SearchKind) -> Result<Vec<Patient>, ApiError> {
    invoke_raw(
        "search_patients",
        serde_json::json!({ "query": query, "kind": kind }),
    )
    .await
}

/// Load and analyze today's orders.
pub async fn load_review(
    hn: &str,
    pregnancy_override: Option<PregnancyStatus>,
    weight_override: Option<f64>,
) -> Result<ReviewResult, ApiError> {
    invoke_raw(
        "load_review",
        serde_json::json!({
            "hn": hn,
            "pregnancyOverride": pregnancy_override,
            "weightOverride": weight_override,
        }),
    )
    .await
}
