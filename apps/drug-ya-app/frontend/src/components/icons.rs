//! Inline SVG icons. One generic line-icon component keeps the bundle small.

use leptos::prelude::*;

/// The Drug-Ya mark (the supplied icon, without the outer shadow).
#[component]
pub fn IconLogo(#[prop(optional, into)] class: String) -> impl IntoView {
    view! {
        <svg class=class viewBox="0 0 512 512" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
            <rect x="16" y="16" width="480" height="480" rx="112" fill="#1A1C1A" stroke="#556B2F" stroke-width="8"/>
            <path d="M 80 140 V 80 A 16 16 0 0 1 96 64 H 140" fill="none" stroke="#8FBC6A" stroke-width="16" stroke-linecap="round"/>
            <path d="M 432 140 V 80 A 16 16 0 0 0 416 64 H 372" fill="none" stroke="#8FBC6A" stroke-width="16" stroke-linecap="round"/>
            <path d="M 80 372 V 432 A 16 16 0 0 0 96 448 H 140" fill="none" stroke="#8FBC6A" stroke-width="16" stroke-linecap="round"/>
            <path d="M 432 372 V 432 A 16 16 0 0 1 416 448 H 372" fill="none" stroke="#8FBC6A" stroke-width="16" stroke-linecap="round"/>
            <rect x="106" y="186" width="300" height="140" rx="70" fill="#FFFFFF"/>
            <path d="M 256 186 V 326" stroke="#1A1C1A" stroke-width="12"/>
            <path d="M 256 186 H 336 A 70 70 0 0 1 406 256 A 70 70 0 0 1 336 326 H 256 Z" fill="#556B2F"/>
        </svg>
    }
}

/// One line icon.
#[component]
pub fn Icon(name: &'static str, #[prop(optional, into)] class: String) -> impl IntoView {
    let path = match name {
        "search" => "M21 21l-4.35-4.35M10.5 18a7.5 7.5 0 1 1 0-15 7.5 7.5 0 0 1 0 15z",
        "settings" => "M4 7h10M18 7h2M4 17h2M10 17h10M14 4v6M8 14v6",
        "logout" => "M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4M16 17l5-5-5-5M21 12H9",
        "refresh" => "M21 12a9 9 0 1 1-2.64-6.36M21 3v6h-6",
        "shield" => "M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z",
        "check" => "M22 11.1V12a10 10 0 1 1-5.9-9.1M22 4L12 14l-3-3",
        "alert" => {
            "M10.3 3.9L1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0zM12 9v4M12 17h.01"
        }
        "info" => "M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20zM12 16v-4M12 8h.01",
        "user" => "M20 21a8 8 0 0 0-16 0M12 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8z",
        "clock" => "M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20zM12 6v6l4 2",
        "flask" => "M9 3h6M10 3v6L4.5 19a2 2 0 0 0 1.8 3h11.4a2 2 0 0 0 1.8-3L14 9V3M7 15h10",
        "kidney" => {
            "M9.5 4.5c-2.7 0-4.5 2.4-4.5 5.7 0 4.5 2.1 9.3 5 9.3 1.8 0 2.8-1.6 2.8-3.6 0-2.7-1.3-4.4-1.3-6.4 0-2.7-.5-5-2-5zM14.5 19.5c2.7 0 4.5-2.4 4.5-5.7 0-4.5-2.1-9.3-5-9.3-1.8 0-2.8 1.6-2.8 3.6 0 2.7 1.3 4.4 1.3 6.4 0 2.7.5 5 2 5z"
        }
        "pill" => "M10.5 20.5l10-10a4.95 4.95 0 0 0-7-7l-10 10a4.95 4.95 0 0 0 7 7zM8.5 8.5l7 7",
        "interaction" => "M8 3v4M8 17v4M4 8h4M17 4l3 3-6 6-3-3 6-6zM14 14l6 6M17 20l3-3",
        "baby" => "M12 8a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM6 21v-5a6 6 0 0 1 12 0v5M9 21v-3M15 21v-3",
        "pregnancy" => {
            "M12 7a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5zM9 22v-5l-2-4 2-4h4l2 4-1 3h2a3 3 0 0 1 3 3v3"
        }
        "heart" => {
            "M20.8 4.6a5.5 5.5 0 0 0-7.8 0L12 5.7l-1.1-1.1a5.5 5.5 0 0 0-7.8 7.8l1.1 1.1L12 21l7.8-7.5 1.1-1.1a5.5 5.5 0 0 0-.1-7.8z"
        }
        "chevron" => "M9 18l6-6-6-6",
        "filter" => "M22 3H2l8 9.5V19l4 2v-8.5L22 3z",
        "close" => "M18 6L6 18M6 6l12 12",
        "lock" => "M5 11h14v10H5zM8 11V7a4 4 0 0 1 8 0v4",
        "eye" => {
            "M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12zM12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z"
        }
        "eye-off" => {
            "M3 3l18 18M10.6 10.6a2 2 0 0 0 2.8 2.8M9.9 5.2A9.8 9.8 0 0 1 12 5c6.5 0 10 7 10 7a17 17 0 0 1-2.4 3.2M6.3 6.3A17 17 0 0 0 2 12s3.5 7 10 7a9.7 9.7 0 0 0 4-.8"
        }
        "database" => {
            "M12 3c4.4 0 8 1.3 8 3s-3.6 3-8 3-8-1.3-8-3 3.6-3 8-3zM4 6v12c0 1.7 3.6 3 8 3s8-1.3 8-3V6M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3"
        }
        "leaf" => "M20 4C10 4 4 9 4 17c0 1 .2 2 .5 3C7 13 12 9 19 8c-5 3-8 7-9 12 9 1 14-5 14-16z",
        "spark" => {
            "M12 2l1.8 5.2L19 9l-5.2 1.8L12 16l-1.8-5.2L5 9l5.2-1.8L12 2zM19 16l.9 2.1L22 19l-2.1.9L19 22l-.9-2.1L16 19l2.1-.9L19 16z"
        }
        _ => "M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20z",
    };
    view! {
        <svg class=class viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d=path/>
        </svg>
    }
}
