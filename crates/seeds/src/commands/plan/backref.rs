//! The `seeds:plan-backref` block plan children carry in their
//! description (sd 0.5.15 `plan-backref.ts`): build, replace, strip.

/// The block's opening marker.
pub(crate) const BACKREF_START: &str = "<!-- seeds:plan-backref:start -->";
/// The block's closing marker.
pub(crate) const BACKREF_END: &str = "<!-- seeds:plan-backref:end -->";

const APPROACH_EXCERPT_MAX: usize = 240;

/// The fields the backref block carries.
#[derive(Clone, Debug)]
pub(crate) struct BackrefArgs<'a> {
    /// 0-based step index; `None` for loose adoptions.
    pub step_index:        Option<usize>,
    /// The plan id.
    pub plan_id:           &'a str,
    /// The parent seed id.
    pub parent_seed_id:    &'a str,
    /// The parent seed title.
    pub parent_seed_title: &'a str,
    /// The plan's template name.
    pub template_name:     &'a str,
    /// The plan's approach section (excerpted).
    pub approach:          Option<&'a Value>,
}

use serde_json::Value;

/// Builds a fresh backref block.
pub(crate) fn build_plan_backref(args: &BackrefArgs<'_>) -> String {
    let mut lines = Vec::new();
    match args.step_index {
        Some(index) => lines.push(format!("Step {} of plan {}.", index + 1, args.plan_id)),
        None => lines.push(format!("Adopted into plan {}.", args.plan_id)),
    }
    lines.push(String::new());
    lines.push(format!(
        "Parent seed: {} — {}",
        args.parent_seed_id, args.parent_seed_title
    ));
    lines.push(format!("Plan template: {}", args.template_name));
    if let Some(excerpt) = approach_excerpt(args.approach) {
        lines.push(format!("Plan approach: {excerpt}"));
    }
    lines.push(String::new());
    lines.push(format!(
        "Run `sd plan show {}` for the full plan (context, alternatives, sibling steps, \
         acceptance criteria).",
        args.plan_id
    ));
    format!("{BACKREF_START}\n{}\n{BACKREF_END}", lines.join("\n"))
}

/// Replaces the marker block when present, else prepends a fresh one
/// in front of the existing description.
pub(crate) fn apply_plan_backref(existing: Option<&str>, args: &BackrefArgs<'_>) -> String {
    let block = build_plan_backref(args);
    let prior = existing.unwrap_or_default();
    if has_markers(prior) {
        return replace_section(prior, &block);
    }
    if prior.trim().is_empty() {
        return block;
    }
    format!("{block}\n\n{prior}")
}

/// Strips the marker block; `None` when nothing but the block
/// remained, so the caller drops the description field entirely.
pub(crate) fn strip_plan_backref(existing: &str) -> Option<String> {
    if !has_markers(existing) {
        return Some(existing.to_owned());
    }
    let start = existing.find(BACKREF_START)?;
    let end = existing.find(BACKREF_END)?;
    let before = existing[..start].trim_end();
    let after = existing[end + BACKREF_END.len()..].trim_start();
    match (before.is_empty(), after.is_empty()) {
        (true, true) => None,
        (true, false) => Some(after.to_owned()),
        (false, true) => Some(before.to_owned()),
        (false, false) => Some(format!("{before}\n\n{after}")),
    }
}

fn has_markers(text: &str) -> bool {
    text.contains(BACKREF_START) && text.contains(BACKREF_END)
}

fn replace_section(existing: &str, block: &str) -> String {
    let Some(start) = existing.find(BACKREF_START) else {
        return block.to_owned();
    };
    let Some(end) = existing.find(BACKREF_END) else {
        return block.to_owned();
    };
    let before = &existing[..start];
    let after = &existing[end + BACKREF_END.len()..];
    format!("{before}{block}{after}")
}

fn approach_excerpt(value: Option<&Value>) -> Option<String> {
    let text = value?.as_str()?;
    let collapsed: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let chars: Vec<char> = collapsed.chars().collect();
    if chars.is_empty() {
        return None;
    }
    if chars.len() <= APPROACH_EXCERPT_MAX {
        return Some(chars.into_iter().collect());
    }
    let slice: String = chars[..APPROACH_EXCERPT_MAX].iter().collect();
    let cut = slice
        .rfind(' ')
        .filter(|space| *space > APPROACH_EXCERPT_MAX / 2)
        .map_or(slice.clone(), |space| slice[..space].to_owned());
    Some(format!("{}…", cut.trim_end()))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn args(approach: Option<&Value>) -> BackrefArgs<'_> {
        BackrefArgs {
            step_index: Some(0),
            plan_id: "pl-1234",
            parent_seed_id: "tst-0001",
            parent_seed_title: "Parent",
            template_name: "feature",
            approach,
        }
    }

    #[test]
    fn builds_step_one_block() {
        let block = build_plan_backref(&args(None));
        assert!(block.starts_with(BACKREF_START));
        assert!(block.contains("Step 1 of plan pl-1234.\n"));
        assert!(block.contains("Parent seed: tst-0001 — Parent"));
        assert!(!block.contains("Plan approach:"));
    }

    #[test]
    fn replaces_existing_block_and_keeps_manual_notes() {
        let first = apply_plan_backref(Some("manual note"), &args(None));
        assert!(first.ends_with("\n\nmanual note"));
        let second = apply_plan_backref(Some(&first), &args(Some(&json!("New approach"))));
        assert!(second.contains("Plan approach: New approach"));
        assert!(second.ends_with("manual note"));
        assert_eq!(second.matches(BACKREF_START).count(), 1);
    }

    #[test]
    fn strips_to_none_when_only_the_block() {
        let block = build_plan_backref(&args(None));
        assert_eq!(strip_plan_backref(&block), None);
        let wrapped = format!("before\n\n{block}\n\nafter");
        assert_eq!(
            strip_plan_backref(&wrapped).as_deref(),
            Some("before\n\nafter")
        );
    }

    #[test]
    fn excerpts_long_approaches_at_word_boundary() {
        let long = "word ".repeat(60);
        let excerpt = approach_excerpt(Some(&json!(long))).expect("excerpt");
        assert!(excerpt.len() < long.len() + 10);
        assert!(excerpt.ends_with('…'));
    }
}
