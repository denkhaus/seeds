//! Native Rust implementation of the seeds git-native issue-tracker format.
//!
//! Format compatibility contract (the README format-compatibility
//! contract):
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
//!
//! Mutating commands run under an advisory writer lock on
//! `.seeds/.lock` ([`WriterLock`], seeds-0d7b): one load→mutate→save
//! cycle at a time per store, with the kernel releasing the lock on
//! holder death. Readers stay lock-free.

pub mod commands;
mod config;
mod dedupe;
mod doctor;
mod error;
mod id;
mod lock;
mod model;
mod pace;
mod render;
mod store;
mod timeutil;

pub use config::{Config, VcsManager};
pub use error::{Error, IdError, RecordError};
pub use id::{PlanId, SeedId, TemplateId};
pub use lock::WriterLock;
pub use model::{Fields, PlanRecord, Priority, SeedRecord, SeedType, Status, TemplateRecord};
pub use pace::{DEFAULT_MIN_CLOSURES, DEFAULT_WINDOW_DAYS, Pace, PaceOptions, WeekBucket};
pub use store::Store;
