//! Tauri command adapters.
//!
//! Errors cross the IPC as a typed `CommandError` (kind + Thai message). This
//! layer is the only place where backend errors are translated for the UI.
//! No patient identifiers or drug names are logged.

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use chrono::Local;
use drug_ya_config::{ConnectionConfig, SiteSettings};
use drug_ya_core::model::{PregnancyStatus, ReviewResult, RulePackInfo};
use drug_ya_core::query_kind::{SearchKind, parse_search};
use drug_ya_core::rules::{ReviewSettings, RuleConfigView, RulePackSignoff, default_rule_pack};
use drug_ya_core::{SessionUser, thai, verify_password};
use drug_ya_hosxp::{HosxpClient, HosxpConfig, ReviewRequest};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::AppState;

/// In-memory vault used when the OS keychain is unavailable. Configuration
/// saved through it is never persisted to disk.
#[derive(Default)]
pub struct EphemeralVault {
    key: std::sync::Mutex<Option<encryptman::MasterKey>>,
}

impl drug_ya_config::SecretVault for EphemeralVault {
    fn encrypt(&self, plaintext: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut guard = self.key.lock().expect("invariant: ephemeral key lock");
        let key = guard.get_or_insert_with(|| {
            encryptman::MasterKey::generate().expect("invariant: OS rng available")
        });
        Ok(encryptman::encrypt(key, plaintext)?)
    }

    fn decrypt(
        &self,
        ciphertext: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let guard = self.key.lock().expect("invariant: ephemeral key lock");
        let key = guard
            .as_ref()
            .ok_or_else(|| Box::new(std::io::Error::other("no master key available")))?;
        Ok(encryptman::decrypt(key, ciphertext)?)
    }
}

const LOGIN_FAILURE_THRESHOLD: u32 = 5;
const LOGIN_LOCK_BASE_SECS: u64 = 30;
const LOGIN_LOCK_MAX_STEPS: u32 = 4;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(15);

/// Failure class of a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CommandErrorKind {
    /// No connection settings stored.
    NotConfigured,
    /// HOSxP could not be reached.
    Connection,
    /// The read-only guard rejected a statement.
    Guard,
    /// The statement failed server-side or the data was unusable.
    Query,
    /// The operator is not signed in, or credentials were rejected.
    Auth,
    /// Submitted input is not usable.
    Validation,
}

/// User-facing command error: a machine-readable kind plus a Thai message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    /// Failure class.
    pub kind: CommandErrorKind,
    /// Thai message shown verbatim.
    pub message: String,
}

impl CommandError {
    fn new(kind: CommandErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    fn validation(message: impl Into<String>) -> Self {
        Self::new(CommandErrorKind::Validation, message)
    }

    fn auth(message: impl Into<String>) -> Self {
        Self::new(CommandErrorKind::Auth, message)
    }

    fn unauthorized() -> Self {
        Self::auth("กรุณาเข้าสู่ระบบก่อนใช้งาน")
    }
}

fn dev_log(
    context: &str,
    detail: &impl std::fmt::Debug,
    kind: CommandErrorKind,
    message: &'static str,
) -> CommandError {
    eprintln!("[drug-ya] {context} failed: {detail:?}");
    CommandError::new(kind, message)
}

fn map_repo_error(error: drug_ya_hosxp::Error, action: &'static str) -> CommandError {
    eprintln!("[drug-ya] {action} failed: {error:?}");
    match error {
        drug_ya_hosxp::Error::Connect { .. } => {
            CommandError::new(CommandErrorKind::Connection, "เชื่อมต่อฐานข้อมูล HOSxP ไม่สำเร็จ")
        }
        drug_ya_hosxp::Error::Database(sqlx::Error::PoolTimedOut)
        | drug_ya_hosxp::Error::Database(sqlx::Error::PoolClosed)
        | drug_ya_hosxp::Error::Database(sqlx::Error::Io(_)) => {
            CommandError::new(CommandErrorKind::Connection, "เชื่อมต่อฐานข้อมูล HOSxP ไม่สำเร็จ")
        }
        drug_ya_hosxp::Error::Database(sqlx::Error::Database(_)) => {
            CommandError::new(CommandErrorKind::Query, format!("{action}ไม่สำเร็จ"))
        }
        drug_ya_hosxp::Error::ReadOnlyViolation(_) => {
            CommandError::new(CommandErrorKind::Guard, "ระบบความปลอดภัยของแอปปฏิเสธคำสั่งนี้")
        }
        drug_ya_hosxp::Error::UnsupportedSchema(_) => CommandError::new(
            CommandErrorKind::Query,
            "ฐานข้อมูล HOSxP นี้ยังไม่รองรับคอลัมน์คิว (ovst.oqueue/ovst.queue) - กรุณาค้นด้วย HN, CID หรือชื่อ",
        ),
        drug_ya_hosxp::Error::NotFound(message) => {
            CommandError::new(CommandErrorKind::Query, message)
        }
        other => CommandError::new(
            CommandErrorKind::Query,
            format!("{action}ไม่สำเร็จ ({other})"),
        ),
    }
}

/// Connection settings submitted from the settings dialog.
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
    /// Database password (transient only).
    pub password: String,
}

impl From<ConnectionInput> for ConnectionConfig {
    fn from(input: ConnectionInput) -> Self {
        Self {
            host: input.host.trim().to_string(),
            port: input.port,
            database: input.database.trim().to_string(),
            user: input.user.trim().to_string(),
            password: SecretString::from(input.password),
        }
    }
}

/// Login credentials. Deliberately not `Debug`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginInput {
    /// HOSxP officer login name.
    pub username: String,
    /// Plaintext password, verified in memory.
    pub password: String,
}

/// Non-secret connection summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInfo {
    /// Hostname or IP.
    pub host: String,
    /// TCP port.
    pub port: u16,
    /// Database name.
    pub database: String,
    /// Database user.
    pub user: String,
}

/// Live reachability for the top-bar dot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionHealth {
    /// No saved connection.
    Unconfigured,
    /// Last ping succeeded.
    Connected,
    /// Last ping failed.
    Disconnected,
}

/// `SELECT 1` result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTestResult {
    /// Round-trip latency.
    pub latency_ms: u64,
}

/// App status for the login screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Rule-pack version.
    pub rule_pack_version: String,
    /// Whether the pack has been signed off.
    pub signed_off: bool,
}

/// Settings payload for the review-settings screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewSettingsResponse {
    /// Current settings.
    pub settings: ReviewSettings,
    /// Pack metadata.
    pub pack: RulePackInfo,
    /// Rule rows with effective state.
    pub rules: Vec<RuleConfigView>,
}

/// Sign-off form.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignoffInput {
    /// Pharmacist display name.
    pub pharmacist_name: String,
    /// Professional licence number.
    pub license_no: String,
    /// Optional note.
    pub note: String,
}

fn to_hosxp_config(connection: &ConnectionConfig) -> HosxpConfig {
    HosxpConfig {
        host: connection.host.clone(),
        port: connection.port,
        database: connection.database.clone(),
        user: connection.user.clone(),
        password: connection.password.clone(),
    }
}

fn stored_connection(state: &AppState) -> Result<ConnectionConfig, CommandError> {
    state.store.load_connection().map_err(|_| {
        CommandError::new(CommandErrorKind::NotConfigured, "ยังไม่ได้ตั้งค่าการเชื่อมต่อ HOSxP")
    })
}

fn stored_settings(state: &AppState) -> Result<SiteSettings, CommandError> {
    state.store.load_settings().map_err(|error| {
        dev_log(
            "settings",
            &error,
            CommandErrorKind::Query,
            "อ่านการตั้งค่าไม่สำเร็จ",
        )
    })
}

async fn connect_client(
    config: HosxpConfig,
    action: &'static str,
) -> Result<HosxpClient, CommandError> {
    match tokio::time::timeout(COMMAND_TIMEOUT, HosxpClient::connect(config)).await {
        Ok(Ok(client)) => Ok(client),
        Ok(Err(error)) => Err(map_repo_error(error, action)),
        Err(_) => Err(CommandError::new(
            CommandErrorKind::Connection,
            format!("{action}หมดเวลา - ตรวจสอบ Host/Port และเครือข่าย"),
        )),
    }
}

async fn client(state: &AppState, action: &'static str) -> Result<HosxpClient, CommandError> {
    {
        let guard = state.client.read().await;
        if let Some(client) = guard.as_ref() {
            return Ok(client.clone());
        }
    }
    let connection = stored_connection(state)?;
    let client = connect_client(to_hosxp_config(&connection), action).await?;
    let mut slot = state.client.write().await;
    if slot.is_none() {
        *slot = Some(client.clone());
    }
    Ok(client)
}

async fn require_session(state: &AppState) -> Result<SessionUser, CommandError> {
    state
        .session
        .read()
        .await
        .clone()
        .ok_or_else(CommandError::unauthorized)
}

async fn clear_session(state: &AppState) -> u64 {
    let epoch = state
        .session_epoch
        .fetch_add(1, Ordering::SeqCst)
        .wrapping_add(1);
    *state.session.write().await = None;
    epoch
}

fn login_lockout_duration(failures: u32) -> Duration {
    let steps = failures
        .saturating_sub(LOGIN_FAILURE_THRESHOLD)
        .min(LOGIN_LOCK_MAX_STEPS);
    Duration::from_secs(LOGIN_LOCK_BASE_SECS << steps)
}

async fn login_lockout_remaining(state: &AppState) -> Option<Duration> {
    let now = Instant::now();
    let mut throttle = state.login_throttle.lock().await;
    match throttle.locked_until {
        Some(until) if until > now => Some(until - now),
        Some(_) => {
            throttle.locked_until = None;
            throttle.failed_attempts = 0;
            None
        }
        None => None,
    }
}

async fn record_login_failure(state: &AppState) {
    let mut throttle = state.login_throttle.lock().await;
    throttle.failed_attempts = throttle.failed_attempts.saturating_add(1);
    if throttle.failed_attempts >= LOGIN_FAILURE_THRESHOLD {
        let lockout = login_lockout_duration(throttle.failed_attempts);
        throttle.locked_until = Some(Instant::now() + lockout);
    }
}

async fn reset_login_failures(state: &AppState) {
    let mut throttle = state.login_throttle.lock().await;
    throttle.failed_attempts = 0;
    throttle.locked_until = None;
}

async fn auth_failure(state: &AppState, message: &'static str) -> CommandError {
    record_login_failure(state).await;
    CommandError::auth(message)
}

/// Status for the login screen (no password or PHI).
#[tauri::command]
pub async fn get_app_status(state: State<'_, AppState>) -> Result<AppStatus, CommandError> {
    let settings = stored_settings(&state)?;
    let pack = default_rule_pack();
    let configured = state.store.connection_exists();
    let site_name = settings.review.site_name.clone();
    let signed_off = pack.info(&settings.review).signed_off;
    match state.store.load_connection() {
        Ok(connection) => Ok(AppStatus {
            configured,
            site_name,
            host: Some(connection.host),
            database: Some(connection.database),
            user: Some(connection.user),
            rule_pack_version: pack.version.clone(),
            signed_off,
        }),
        Err(_) => Ok(AppStatus {
            configured: false,
            site_name,
            host: None,
            database: None,
            user: None,
            rule_pack_version: pack.version.clone(),
            signed_off,
        }),
    }
}

/// Whether a connection config exists.
#[tauri::command]
pub async fn is_configured(state: State<'_, AppState>) -> Result<bool, CommandError> {
    Ok(state.store.connection_exists())
}

/// Live connectivity for the top-bar dot.
#[tauri::command]
pub async fn connection_health(
    state: State<'_, AppState>,
) -> Result<ConnectionHealth, CommandError> {
    if !state.store.connection_exists() {
        return Ok(ConnectionHealth::Unconfigured);
    }
    match client(&state, "ตรวจสอบการเชื่อมต่อ").await {
        Ok(client) => match client.ping().await {
            Ok(()) => {
                *state.health.write().await = ConnectionHealth::Connected;
                Ok(ConnectionHealth::Connected)
            }
            Err(_) => {
                *state.health.write().await = ConnectionHealth::Disconnected;
                Ok(ConnectionHealth::Disconnected)
            }
        },
        Err(error) if error.kind == CommandErrorKind::NotConfigured => {
            Ok(ConnectionHealth::Unconfigured)
        }
        Err(_) => Ok(ConnectionHealth::Disconnected),
    }
}

/// Save the connection (encrypted) and connect before persisting.
#[tauri::command]
pub async fn save_connection(
    state: State<'_, AppState>,
    config: ConnectionInput,
) -> Result<(), CommandError> {
    if config.host.trim().is_empty()
        || config.database.trim().is_empty()
        || config.user.trim().is_empty()
    {
        return Err(CommandError::validation(
            "กรุณากรอก Host, Database และ User ให้ครบ",
        ));
    }
    if config.password.is_empty() {
        return Err(CommandError::validation("กรุณากรอกรหัสผ่านฐานข้อมูล"));
    }
    let connection: ConnectionConfig = config.into();
    let client = connect_client(to_hosxp_config(&connection), "บันทึกการตั้งค่า").await?;

    let saved = tokio::time::timeout(COMMAND_TIMEOUT, async {
        state.store.save_connection(&connection)
    })
    .await;
    match saved {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            return Err(dev_log(
                "save_connection",
                &error,
                CommandErrorKind::Query,
                "บันทึกการตั้งค่าไม่สำเร็จ",
            ));
        }
        Err(_) => {
            return Err(CommandError::new(
                CommandErrorKind::Query,
                "บันทึกการตั้งค่าไม่สำเร็จ - การเข้าถึง Keychain ใช้เวลานานเกินไป",
            ));
        }
    }

    clear_session(&state).await;
    let old = {
        let mut slot = state.client.write().await;
        slot.replace(client)
    };
    if let Some(old) = old {
        old.disconnect().await;
    }
    *state.health.write().await = ConnectionHealth::Connected;
    tracing::info!("connection saved");
    Ok(())
}

/// Saved connection summary (password never returned).
#[tauri::command]
pub async fn get_connection(state: State<'_, AppState>) -> Result<ConnectionInfo, CommandError> {
    let connection = stored_connection(&state)?;
    Ok(ConnectionInfo {
        host: connection.host,
        port: connection.port,
        database: connection.database,
        user: connection.user,
    })
}

/// Test connectivity without saving.
#[tauri::command]
pub async fn test_connection(
    state: State<'_, AppState>,
    config: Option<ConnectionInput>,
) -> Result<ConnectionTestResult, CommandError> {
    let connection = match config {
        Some(input) => ConnectionConfig::from(input),
        None => stored_connection(&state)?,
    };
    let started = Instant::now();
    let client = connect_client(to_hosxp_config(&connection), "ทดสอบการเชื่อมต่อ").await?;
    let ping = tokio::time::timeout(COMMAND_TIMEOUT, client.ping()).await;
    match ping {
        Ok(Ok(())) => {}
        Ok(Err(error)) => return Err(map_repo_error(error, "ทดสอบการเชื่อมต่อ")),
        Err(_) => {
            return Err(CommandError::new(
                CommandErrorKind::Connection,
                "ทดสอบการเชื่อมต่อหมดเวลา - ตรวจสอบ Host/Port และเครือข่าย",
            ));
        }
    }
    client.disconnect().await;
    Ok(ConnectionTestResult {
        latency_ms: started.elapsed().as_millis() as u64,
    })
}

/// Remove stored configuration (requires sign-in).
#[tauri::command]
pub async fn clear_connection(state: State<'_, AppState>) -> Result<(), CommandError> {
    require_session(&state).await?;
    *state.client.write().await = None;
    clear_session(&state).await;
    state.store.clear().map_err(|error| {
        dev_log(
            "clear_connection",
            &error,
            CommandErrorKind::Query,
            "ลบการตั้งค่าไม่สำเร็จ",
        )
    })?;
    *state.health.write().await = ConnectionHealth::Unconfigured;
    Ok(())
}

/// Verify a HOSxP officer password and start an in-memory session.
#[tauri::command]
pub async fn login(
    state: State<'_, AppState>,
    input: LoginInput,
) -> Result<SessionUser, CommandError> {
    let username = input.username.trim();
    if username.is_empty() {
        return Err(CommandError::validation("กรุณากรอกชื่อผู้ใช้"));
    }
    if username.chars().count() > 100 {
        return Err(CommandError::validation("ชื่อผู้ใช้ยาวเกินไป"));
    }
    let password = SecretString::from(input.password);
    if password.expose_secret().is_empty() {
        return Err(CommandError::validation("กรุณากรอกรหัสผ่าน"));
    }
    if password.expose_secret().chars().count() > 200 {
        return Err(CommandError::validation("รหัสผ่านยาวเกินไป"));
    }
    if let Some(remaining) = login_lockout_remaining(&state).await {
        return Err(CommandError::auth(format!(
            "เข้าสู่ระบบถูกระงับชั่วคราว กรุณารอ {} วินาทีแล้วลองใหม่",
            remaining.as_secs().max(1)
        )));
    }

    let epoch = clear_session(&state).await;
    let client = client(&state, "เข้าสู่ระบบ").await?;
    let account = client
        .fetch_officer(username)
        .await
        .map_err(|error| map_repo_error(error, "เข้าสู่ระบบ"))?;
    let Some(account) = account else {
        return Err(auth_failure(&state, "ชื่อผู้ใช้หรือรหัสผ่านไม่ถูกต้อง").await);
    };
    if account.auth_type_id != 1 {
        return Err(auth_failure(&state, "บัญชีนี้ใช้การยืนยันตัวตนแบบ LDAP ซึ่งยังไม่เปิดใช้งาน").await);
    }
    if !account.active.eq_ignore_ascii_case("Y") {
        return Err(auth_failure(&state, "บัญชีนี้ถูกปิดใช้งาน กรุณาติดต่อผู้ดูแลระบบ").await);
    }
    if !verify_password(password.expose_secret(), &account.password_md5) {
        return Err(auth_failure(&state, "ชื่อผู้ใช้หรือรหัสผ่านไม่ถูกต้อง").await);
    }

    let display_name = if account.display_name.trim().is_empty() {
        account.login_name.clone()
    } else {
        account.display_name
    };
    let user = SessionUser {
        officer_id: account.officer_id,
        login_name: account.login_name,
        display_name,
        doctor_code: account.doctor_code,
        role: "STAFF".to_string(),
    };
    reset_login_failures(&state).await;
    let mut slot = state.session.write().await;
    if state.session_epoch.load(Ordering::SeqCst) != epoch {
        return Err(CommandError::auth("เซสชันถูกยกเลิก กรุณาเข้าสู่ระบบอีกครั้ง"));
    }
    *slot = Some(user.clone());
    Ok(user)
}

/// Clear the in-memory session.
#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<(), CommandError> {
    clear_session(&state).await;
    Ok(())
}

/// Current session, if any.
#[tauri::command]
pub async fn current_session(
    state: State<'_, AppState>,
) -> Result<Option<SessionUser>, CommandError> {
    Ok(state.session.read().await.clone())
}

/// Current review settings + rule catalog.
#[tauri::command]
pub async fn get_review_settings(
    state: State<'_, AppState>,
) -> Result<ReviewSettingsResponse, CommandError> {
    require_session(&state).await?;
    let settings = stored_settings(&state)?;
    let pack = default_rule_pack();
    Ok(ReviewSettingsResponse {
        pack: pack.info(&settings.review),
        rules: pack.config_views(&settings.review),
        settings: settings.review,
    })
}

/// Save review settings (non-secret, no patient data).
#[tauri::command]
pub async fn save_review_settings(
    state: State<'_, AppState>,
    settings: ReviewSettings,
) -> Result<(), CommandError> {
    require_session(&state).await?;
    let mut settings = settings;
    // Rule changes must not silently replace the pharmacist's signature. The
    // only path that writes `signoff` is `sign_off_rule_pack`.
    settings.signoff = stored_settings(&state)?.review.signoff;
    settings.normalize();
    if settings.site_name.chars().count() > 200 {
        return Err(CommandError::validation("ชื่อสถานบริการยาวเกินไป"));
    }
    if settings.overrides.len() > 500 {
        return Err(CommandError::validation("จำนวนรายการปรับกฎมากเกินไป"));
    }
    let pack = default_rule_pack();
    for (rule_id, override_value) in &settings.overrides {
        let Some(rule) = pack.rules.iter().find(|rule| &rule.id == rule_id) else {
            return Err(CommandError::validation("พบรหัสกฎที่ไม่รู้จักในการตั้งค่า"));
        };
        if let (Some(threshold), Some(meta)) = (override_value.threshold, rule.threshold_meta())
            && (!threshold.is_finite() || threshold < meta.min || threshold > meta.max)
        {
            return Err(CommandError::validation(format!(
                "ค่าเกณฑ์ของกฎ {} อยู่นอกช่วงที่อนุญาต",
                rule.title
            )));
        }
    }
    state
        .store
        .save_settings(&SiteSettings { review: settings })
        .map_err(|error| {
            dev_log(
                "save_review_settings",
                &error,
                CommandErrorKind::Query,
                "บันทึกการตั้งค่าไม่สำเร็จ",
            )
        })
}

/// Sign off the current rule-pack version.
#[tauri::command]
pub async fn sign_off_rule_pack(
    state: State<'_, AppState>,
    input: SignoffInput,
) -> Result<RulePackInfo, CommandError> {
    require_session(&state).await?;
    let name = input.pharmacist_name.trim();
    let license = input.license_no.trim();
    if name.chars().count() < 3 {
        return Err(CommandError::validation("กรุณากรอกชื่อเภสัชกรผู้รับรอง"));
    }
    if license.chars().count() < 3 {
        return Err(CommandError::validation("กรุณากรอกเลขใบประกอบวิชาชีพ"));
    }
    if input.note.chars().count() > 1000 {
        return Err(CommandError::validation("หมายเหตุยาวเกินไป"));
    }
    let mut settings = stored_settings(&state)?;
    let pack = default_rule_pack();
    settings.review.signoff = Some(RulePackSignoff {
        pharmacist_name: name.to_string(),
        license_no: license.to_string(),
        signed_on: Local::now().date_naive(),
        pack_version: pack.version.clone(),
        note: input.note.trim().to_string(),
    });
    state.store.save_settings(&settings).map_err(|error| {
        dev_log(
            "sign_off_rule_pack",
            &error,
            CommandErrorKind::Query,
            "บันทึกการรับรองไม่สำเร็จ",
        )
    })?;
    Ok(pack.info(&settings.review))
}

/// Search patients by HN, CID, name, or today's queue.
#[tauri::command]
pub async fn search_patients(
    state: State<'_, AppState>,
    query: String,
    kind: SearchKind,
) -> Result<Vec<drug_ya_core::Patient>, CommandError> {
    require_session(&state).await?;
    if query.chars().count() > 100 {
        return Err(CommandError::validation("คำค้นหายาวเกินไป"));
    }
    let parsed = parse_search(&query, kind);
    if parsed.term.trim().is_empty() {
        return Ok(Vec::new());
    }
    let client = client(&state, "ค้นหาผู้ป่วย").await?;
    let results = client
        .search_patients(&parsed)
        .await
        .map_err(|error| map_repo_error(error, "ค้นหาผู้ป่วย"))?;
    tracing::debug!(
        kind = ?parsed.kind,
        count = results.len(),
        "patient search"
    );
    Ok(results)
}

/// Load and analyze today's orders for one patient.
#[tauri::command]
pub async fn load_review(
    state: State<'_, AppState>,
    hn: String,
    pregnancy_override: Option<PregnancyStatus>,
    weight_override: Option<f64>,
) -> Result<ReviewResult, CommandError> {
    require_session(&state).await?;
    let hn = hn.trim();
    if hn.is_empty() || hn.chars().count() > 20 {
        return Err(CommandError::validation("HN ไม่ถูกต้อง"));
    }
    if let Some(weight) = weight_override
        && (!weight.is_finite() || !(1.0..=400.0).contains(&weight))
    {
        return Err(CommandError::validation("น้ำหนักตัวไม่ถูกต้อง"));
    }

    let settings = stored_settings(&state)?;
    let pack = default_rule_pack();
    let client = client(&state, "โหลดรายการยา").await?;
    let request = ReviewRequest {
        hn: hn.to_string(),
        today: Local::now().date_naive(),
        pregnancy_override,
        weight_override,
    };
    let dataset = client
        .load_review_dataset(&request)
        .await
        .map_err(|error| map_repo_error(error, "โหลดรายการยา"))?;
    let analysis = drug_ya_core::analyze(
        &dataset.patient,
        &dataset.orders,
        &dataset.clinical,
        &settings.review,
        &pack,
    );
    let now = Local::now();
    let checked_at = format!(
        "{} เวลา {} น.",
        thai::format_thai_date_short(dataset.review_date),
        now.format("%H:%M")
    );
    tracing::debug!(
        orders = dataset.orders.len(),
        findings = analysis.findings.len(),
        "review loaded"
    );
    Ok(ReviewResult {
        patient: dataset.patient,
        review_date: dataset.review_date,
        visits: dataset.visits,
        orders: dataset.orders,
        clinical: dataset.clinical,
        summary: analysis.summary,
        findings: analysis.findings,
        coverage: analysis.coverage,
        warnings: dataset.warnings,
        rule_pack: pack.info(&settings.review),
        checked_at,
    })
}
