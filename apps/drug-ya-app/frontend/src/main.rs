//! Drug-Ya frontend entry point (Leptos 0.8 CSR).
//!
//! This crate is WASM-only. The native target exists so workspace-wide
//! `cargo check` still succeeds; build the UI with `trunk build`.

#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

mod actions;
mod api;
mod app;
mod clock;
mod components;
mod state;

#[cfg(target_arch = "wasm32")]
fn main() {
    app::run();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("drug-ya-frontend is a WASM-only crate; build it with `trunk build`");
}
