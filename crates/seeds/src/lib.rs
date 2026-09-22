//! Native Rust implementation of the seeds git-native issue-tracker format.
//!
//! Format compatibility contract (README, ADR-0023 in denkhaus/fabro):
//! read+write compatible with `@os-eco/seeds-cli` 0.5.15 —
//! `.seeds/config.yaml`, `issues.jsonl`, `plans.jsonl`,
//! `templates.jsonl`; seed ids are `<project>-<hex4>`, dependencies are
//! `blockedBy` arrays on the record; unknown record fields are preserved
//! on every write, and additive fields are the only sanctioned extension
//! mechanism.
//!
//! The store is map-backed: every record keeps its parsed
//! [`serde_json::Map`](serde_json::Map) (with the `preserve_order`
//! feature) as the serialization source of truth, so a load→save cycle
//! reproduces sd's compact JSONL byte for byte, unknown fields included.

pub mod commands;
mod config;
mod dedupe;
mod doctor;
mod error;
mod id;
mod model;
mod render;
mod store;
mod timeutil;

pub use config::Config;
pub use error::{Error, IdError, RecordError};
pub use id::{PlanId, SeedId, TemplateId};
pub use model::{Fields, PlanRecord, Priority, SeedRecord, SeedType, Status, TemplateRecord};
pub use store::Store;
