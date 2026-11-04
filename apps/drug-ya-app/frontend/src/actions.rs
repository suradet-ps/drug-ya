//! Shared async actions used by more than one component.

use leptos::prelude::*;
use leptos::task::spawn_local;

use drug_ya_core::model::{Patient, PregnancyStatus};
use drug_ya_core::rules::ReviewSettings;

use crate::api::{self, SignoffInput};
use crate::state::AppState;

/// Start a debounced search request (the caller owns the debounce).
pub fn start_search(state: AppState) {
    spawn_local(async move {
        run_search(state).await;
    });
}

/// Perform one search request.
pub async fn run_search(state: AppState) {
    let query = state.search_query.get_untracked().trim().to_string();
    let kind = state.search_kind.get_untracked();
    if query.is_empty() {
        state.search_results.set(Vec::new());
        state.search_error.set(None);
        state.search_loading.set(false);
        return;
    }
    let token = state.begin_search();
    state.search_loading.set(true);
    state.search_error.set(None);
    match api::search_patients(&query, kind).await {
        Ok(results) => {
            if state.is_current_search(token) {
                state.search_results.set(results);
                state.search_loading.set(false);
            }
        }
        Err(error) => {
            if state.is_current_search(token) {
                state.search_results.set(Vec::new());
                state.search_error.set(Some(error.message));
                state.search_loading.set(false);
            }
        }
    }
}

/// Select a search hit and load its review.
pub fn select_patient(state: AppState, patient: Patient) {
    state.selected_patient.set(Some(patient));
    state.review.set(None);
    state.review_error.set(None);
    state.pregnancy_override.set(None);
    state.weight_override.set(None);
    state.reset_review_decision();
    reload_review(state);
}

/// Reload the currently selected patient's review.
pub fn reload_review(state: AppState) {
    let Some(patient) = state.selected_patient.get_untracked() else {
        return;
    };
    let Some(session) = state.session.get_untracked() else {
        return;
    };
    if session.login_name.is_empty() {
        return;
    }
    let hn = patient.hn.clone();
    let pregnancy_override = state.pregnancy_override.get_untracked();
    let weight_override = state.weight_override.get_untracked();
    let token = state.begin_review();
    state.review_loading.set(true);
    state.review_error.set(None);
    state.reset_review_decision();
    spawn_local(async move {
        match api::load_review(&hn, pregnancy_override, weight_override).await {
            Ok(review) => {
                if state.is_current_review(token) {
                    state.selected_patient.set(Some(review.patient.clone()));
                    state.review.set(Some(review));
                    state.review_loading.set(false);
                }
            }
            Err(error) => {
                if state.is_current_review(token) {
                    state.review_error.set(Some(error.message));
                    state.review_loading.set(false);
                }
            }
        }
    });
}

/// Set a session-only pregnancy status and re-run the review.
pub fn set_pregnancy(state: AppState, status: PregnancyStatus) {
    state.pregnancy_override.set(Some(status));
    reload_review(state);
}

/// Load the settings/rules payload.
pub async fn load_settings(state: AppState) {
    if state.session.get_untracked().is_none() {
        return;
    }
    state.settings_loading.set(true);
    state.settings_error.set(None);
    match api::get_review_settings().await {
        Ok(data) => {
            state.settings_data.set(Some(data));
            state.settings_loading.set(false);
        }
        Err(error) => {
            state.settings_error.set(Some(error.message));
            state.settings_loading.set(false);
        }
    }
}

/// Save settings, reload them, and refresh an open review.
pub fn save_settings(state: AppState, settings: ReviewSettings) {
    spawn_local(async move {
        state.settings_loading.set(true);
        state.settings_error.set(None);
        match api::save_review_settings(&settings).await {
            Ok(()) => {
                load_settings(state).await;
                if state.review.get_untracked().is_some() {
                    reload_review(state);
                }
            }
            Err(error) => {
                state.settings_error.set(Some(error.message));
                state.settings_loading.set(false);
            }
        }
    });
}

/// Sign off the rule pack.
pub fn signoff(state: AppState, input: SignoffInput) {
    spawn_local(async move {
        state.settings_loading.set(true);
        state.settings_error.set(None);
        match api::sign_off_rule_pack(&input).await {
            Ok(_) => {
                load_settings(state).await;
            }
            Err(error) => {
                state.settings_error.set(Some(error.message));
                state.settings_loading.set(false);
            }
        }
    });
}

/// Sign out (backend + frontend state).
pub fn logout(state: AppState) {
    spawn_local(async move {
        let _ = api::logout().await;
        state.clear_session();
    });
}
