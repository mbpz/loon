//! Application-level modules that wrap `loon-core` stores with
//! high-level CRUD APIs.
//!
//! ## Status (2026-06-19): SCAFFOLDED, NOT WIRED
//!
//! Every `XxxAppModule` here is reachable from `crate::loon_app_modules`
//! but **has zero call-sites** anywhere in the workspace. The engine,
//! SDK, and server routes all call `loon-core` stores directly and
//! bypass this layer entirely. The 20 internal tests in this crate
//! pass, but they only verify the thin `AppModule -> Store` delegation
//! in isolation; no production code path exercises them.
//!
//! This module is kept because (a) it captures the intended layering
//! between `loon-core` and downstream consumers, and (b) the
//! `FakeXxxStore` test fakes here are useful patterns that future
//! `loon-core` tests can adopt. See `docs/adr/` (TODO) for the
//! decision on whether to either wire this layer in (replacing the
//! direct store usage in `loon-server/src/routes/`) or delete it.
//!
//! Until that decision lands, treat this crate as documentation of an
//! intended architecture, not as a live dependency.
pub mod agents;
pub mod canned_responses;
pub mod capabilities;
pub mod context_variables;
pub mod customers;
pub mod evaluations;
pub mod glossary;
pub mod guidelines;
pub mod journeys;
pub mod relationships;
pub mod services;
pub mod sessions;
pub mod tags;
