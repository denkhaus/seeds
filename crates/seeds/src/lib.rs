//! Native Rust implementation of the seeds git-native issue-tracker format.
//!
//! Format compatibility contract (README, ADR-0023 in denkhaus/fabro):
//! read+write compatible with `@os-eco/seeds-cli` 0.5.15 —
//! `.seeds/config.yaml`, `issues.jsonl`, `plans.jsonl`,
//! `templates.jsonl`; seed ids are `<project>-<hex4>`, dependencies are
//! `blockedBy` arrays on the record; unknown record fields are preserved
//! on every write, and additive fields are the only sanctioned extension
//! mechanism.

#[cfg(test)]
mod tests {
    #[test]
    fn workspace_smoke() {
        // Bootstrap placeholder: the format core (reader/writer +
        // round-trip suite) lands as the first product seed.
        assert_eq!(2 + 2, 4);
    }
}
