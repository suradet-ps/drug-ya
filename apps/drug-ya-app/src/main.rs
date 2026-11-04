//! Drug-Ya desktop entry point.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    drug_ya_app::run();
}
