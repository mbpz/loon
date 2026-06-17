//! Core domain types for loon.
//!
//! Stage 1 complete: 15 entities (agent, tag, customer, guideline, journey,
//! observation, session, tool, glossary, variable, canned_response,
//! capability, retriever, relationship, shot), all store traits, and CQ
//! (`EntityQueries` / `EntityCommands`) are exported from the crate root
//! so downstream crates can write `use loon_core::*;`.
pub mod agent;
pub mod async_utils;
pub mod basic_tracer;
pub mod canned_response;
pub mod capability;
pub mod common;
pub mod console_logger;
pub mod customer;
pub mod entity_cq;
pub mod error;
pub mod glossary;
pub mod guideline;
pub mod guideline_tool;
pub mod id_generator;
pub mod ids;
pub mod mcp_client;
pub mod journey;
pub mod journey_guideline_projection;
pub mod logger;
pub mod macros;
pub mod meter;
pub mod observation;
pub mod relationship;
pub mod service_registry;
pub mod session;
pub mod shot;
pub mod stores;
pub mod tag;
pub mod tool;
pub mod tool_insights;
pub mod tool_service;
pub mod tracer;
pub mod variable;
pub use agent::*;
pub use async_utils::*;
pub use canned_response::*;
pub use capability::*;
pub use common::*;
pub use customer::*;
pub use error::*;
pub use glossary::*;
pub use guideline::*;
pub use guideline_tool::*;
pub use id_generator::*;
pub use ids::*;
pub use mcp_client::*;
pub use journey::*;
pub use logger::*;
pub use observation::*;
pub use relationship::*;
pub use service_registry::*;
pub use session::*;
pub use shot::*;
pub use tag::*;
pub use tool::*;
pub use tool_insights::*;
pub use tool_service::*;
pub use tracer::*;
pub use variable::*;
