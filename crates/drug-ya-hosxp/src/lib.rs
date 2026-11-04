//! Read-only HOSxP repository.
//!
//! This crate is the only door to the HOSxP database. Every statement is a
//! compile-time constant or a `?`-placeholder list, every execution passes the
//! read-only guard, every pooled connection starts with `SET SESSION
//! TRANSACTION READ ONLY`, and the view is today's OPD orders plus the
//! clinical context needed to review them.

pub mod client;
pub mod config;
pub mod error;
pub mod queries;
pub mod readonly;

pub use client::{HosxpClient, ReviewRequest};
pub use config::HosxpConfig;
pub use error::Error;
