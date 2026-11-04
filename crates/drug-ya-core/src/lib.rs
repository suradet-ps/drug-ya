//! Drug-Ya pure domain layer - no I/O, no SQL.
//!
//! The domain is a pharmacist-facing review of **today's medication orders**
//! for one patient: identity, today's visits and orders, allergies,
//! diagnoses, recent labs, a deterministic rule engine, and a coverage report
//! that says what could and could not be checked.
//!
//! Clinical rules are data-driven and site-configurable. The bundled pack is
//! a starter set: it must be reviewed and signed off by the site pharmacist
//! before it is used for real dispensing decisions.

pub mod auth;
pub mod dictionary;
pub mod dose;
pub mod engine;
pub mod eras;
pub mod model;
pub mod query_kind;
pub mod renal;
pub mod rules;
pub mod sig;
pub mod thai;

pub use auth::{OfficerAccount, SessionUser, md5_hex, verify_password};
pub use dictionary::{DrugProfile, DrugResolution, class_label, resolve_drug};
pub use engine::analyze;
pub use eras::{buddhist_to_christian, is_buddhist_era_year, normalize_date};
pub use model::*;
pub use query_kind::{ParsedSearch, SearchKind, parse_search};
pub use rules::{
    Category, CheckStatus, CompareOp, DrugSelector, FindingSort, PatientCondition, RenalMetric,
    ReviewSettings, Rule, RuleConfigView, RuleOverride, RulePack, RulePackSignoff, Severity,
    ThresholdMeta, Trigger, default_rule_pack,
};
