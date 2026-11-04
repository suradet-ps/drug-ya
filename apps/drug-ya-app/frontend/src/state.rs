//! Client-side application state (signals shared between components).

use std::collections::HashSet;

use leptos::prelude::*;

use crate::api::{AppStatus, ConnectionHealth, ReviewSettingsResponse};
use drug_ya_core::model::{Patient, PregnancyStatus, ReviewResult};
use drug_ya_core::query_kind::SearchKind;

/// Settings modal tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    /// HOSxP connection.
    Connection,
    /// Rule pack / thresholds.
    Rules,
    /// Pharmacist sign-off.
    Signoff,
}

/// Finding filter in the review panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingFilter {
    /// All findings.
    All,
    /// Critical + high.
    Action,
    /// Moderate + low.
    Monitor,
    /// Informational / high-alert.
    Info,
}

impl FindingFilter {
    /// Thai label.
    pub fn label_th(self) -> &'static str {
        match self {
            Self::All => "ทั้งหมด",
            Self::Action => "ต้องจัดการ",
            Self::Monitor => "เฝ้าระวัง",
            Self::Info => "ข้อมูล",
        }
    }
}

/// Shared state for the single-page review console. `Copy` because every
/// field is a copyable signal.
#[derive(Debug, Clone, Copy)]
pub struct AppState {
    /// Whether an encrypted connection exists.
    pub configured: RwSignal<bool>,
    /// Login-screen status (site label, pack version).
    pub app_status: RwSignal<Option<AppStatus>>,
    /// Signed-in officer.
    pub session: RwSignal<Option<drug_ya_core::SessionUser>>,
    /// True after the startup probe finishes.
    pub auth_checked: RwSignal<bool>,
    /// Live connection health.
    pub health: RwSignal<ConnectionHealth>,
    /// Settings modal open.
    pub settings_open: RwSignal<bool>,
    /// Active settings tab.
    pub settings_tab: RwSignal<SettingsTab>,
    /// Loaded settings + rule catalog.
    pub settings_data: RwSignal<Option<ReviewSettingsResponse>>,
    /// Settings loading.
    pub settings_loading: RwSignal<bool>,
    /// Settings error.
    pub settings_error: RwSignal<Option<String>>,

    /// Search kind.
    pub search_kind: RwSignal<SearchKind>,
    /// Search input.
    pub search_query: RwSignal<String>,
    /// Search in flight.
    pub search_loading: RwSignal<bool>,
    /// Search results.
    pub search_results: RwSignal<Vec<Patient>>,
    /// Search error.
    pub search_error: RwSignal<Option<String>>,
    /// Search request token.
    pub search_epoch: RwSignal<u64>,

    /// Selected patient (until the review loads).
    pub selected_patient: RwSignal<Option<Patient>>,
    /// Loaded review.
    pub review: RwSignal<Option<ReviewResult>>,
    /// Review loading.
    pub review_loading: RwSignal<bool>,
    /// Review error.
    pub review_error: RwSignal<Option<String>>,
    /// Review request token.
    pub review_epoch: RwSignal<u64>,
    /// Session-only pregnancy override.
    pub pregnancy_override: RwSignal<Option<PregnancyStatus>>,
    /// Session-only weight override.
    pub weight_override: RwSignal<Option<f64>>,

    /// Finding IDs acknowledged in this session.
    pub acknowledged: RwSignal<HashSet<String>>,
    /// Finding filter.
    pub finding_filter: RwSignal<FindingFilter>,
    /// Pharmacist's session-local note.
    pub reviewer_note: RwSignal<String>,
    /// Confirmation timestamp (session only).
    pub review_confirmed: RwSignal<Option<String>>,
}

impl AppState {
    /// Fresh state.
    pub fn new() -> Self {
        Self {
            configured: RwSignal::new(false),
            app_status: RwSignal::new(None),
            session: RwSignal::new(None),
            auth_checked: RwSignal::new(false),
            health: RwSignal::new(ConnectionHealth::Unconfigured),
            settings_open: RwSignal::new(false),
            settings_tab: RwSignal::new(SettingsTab::Connection),
            settings_data: RwSignal::new(None),
            settings_loading: RwSignal::new(false),
            settings_error: RwSignal::new(None),
            search_kind: RwSignal::new(SearchKind::Auto),
            search_query: RwSignal::new(String::new()),
            search_loading: RwSignal::new(false),
            search_results: RwSignal::new(Vec::new()),
            search_error: RwSignal::new(None),
            search_epoch: RwSignal::new(0),
            selected_patient: RwSignal::new(None),
            review: RwSignal::new(None),
            review_loading: RwSignal::new(false),
            review_error: RwSignal::new(None),
            review_epoch: RwSignal::new(0),
            pregnancy_override: RwSignal::new(None),
            weight_override: RwSignal::new(None),
            acknowledged: RwSignal::new(HashSet::new()),
            finding_filter: RwSignal::new(FindingFilter::All),
            reviewer_note: RwSignal::new(String::new()),
            review_confirmed: RwSignal::new(None),
        }
    }

    /// Complete a successful login.
    pub fn finish_login(&self, user: drug_ya_core::SessionUser) {
        self.session.set(Some(user));
        self.settings_open.set(false);
        self.configured.set(true);
    }

    /// Clear the session and every patient-scoped signal.
    pub fn clear_session(&self) {
        self.session.set(None);
        self.settings_open.set(false);
        self.settings_data.set(None);
        self.search_query.set(String::new());
        self.search_results.set(Vec::new());
        self.search_error.set(None);
        self.search_loading.set(false);
        self.clear_patient();
    }

    /// Clear the selected patient's review state.
    pub fn clear_patient(&self) {
        self.selected_patient.set(None);
        self.review.set(None);
        self.review_loading.set(false);
        self.review_error.set(None);
        self.pregnancy_override.set(None);
        self.weight_override.set(None);
        self.acknowledged.set(HashSet::new());
        self.finding_filter.set(FindingFilter::All);
        self.reviewer_note.set(String::new());
        self.review_confirmed.set(None);
        self.review_epoch
            .update(|epoch| *epoch = epoch.wrapping_add(1));
    }

    /// Begin a search request and return its token.
    pub fn begin_search(&self) -> u64 {
        let token = self.search_epoch.get_untracked().wrapping_add(1);
        self.search_epoch.set(token);
        token
    }

    /// Whether a search result is still current.
    pub fn is_current_search(&self, token: u64) -> bool {
        self.search_epoch.get_untracked() == token
    }

    /// Begin a review request and return its token.
    pub fn begin_review(&self) -> u64 {
        let token = self.review_epoch.get_untracked().wrapping_add(1);
        self.review_epoch.set(token);
        token
    }

    /// Whether a review result is still current.
    pub fn is_current_review(&self, token: u64) -> bool {
        self.review_epoch.get_untracked() == token
    }

    /// Reset the session-local acknowledgement/sign-off state.
    pub fn reset_review_decision(&self) {
        self.acknowledged.set(HashSet::new());
        self.reviewer_note.set(String::new());
        self.review_confirmed.set(None);
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
