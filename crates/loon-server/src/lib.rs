//! `loon-server` — HTTP / WebSocket server for the `loon` SDK.
//!
//! See module-level docs for each submodule:
//! - [`api`] — common API types (errors, response envelopes).
//! - [`app`] — Axum [`AppState`] and router builder.
//! - [`config`] — TOML-backed server configuration.
//! - [`routes`] — HTTP / WS endpoint handlers.

pub mod api;
pub mod app;
pub mod config;
pub mod routes;

pub use api::common::*;
pub use app::*;
pub use config::*;