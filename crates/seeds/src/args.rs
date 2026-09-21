//! Commander-style argument parsing for the `seeds` binary.
//!
//! Supports `--flag value`, `--flag=value`, short boolean flags, and `--`
//! to treat the rest as positionals — the surface `sd` 0.5.15 (Node
//! commander) accepts.

use std::collections::{HashMap, HashSet};

/// One recognized option: long name and whether it consumes a value.
pub(crate) struct OptSpec {
    /// The canonical long name (without `--`).
    pub long:  &'static str,
    /// Whether the option takes a value (`--flag value` / `--flag=value`).
    pub value: bool,
}

const fn opt(long: &'static str) -> OptSpec {
    OptSpec { long, value: true }
}

const fn flag(long: &'static str) -> OptSpec {
    OptSpec { long, value: false }
}

/// Option specs per command, mirroring the reference's help surface.
pub(crate) static CREATE_SPEC: &[OptSpec] = &[
    opt("title"),
    opt("type"),
    opt("priority"),
    opt("assignee"),
    opt("description"),
    opt("desc"),
    opt("body"),
    opt("labels"),
    flag("json"),
];

pub(crate) static SHOW_SPEC: &[OptSpec] = &[opt("format"), flag("json")];

pub(crate) static LIST_SPEC: &[OptSpec] = &[
    opt("status"),
    opt("type"),
    opt("assignee"),
    flag("all"),
    opt("label"),
    opt("label-any"),
    flag("unlabeled"),
    opt("priority"),
    opt("priority-max"),
    opt("limit"),
    opt("sort"),
    opt("format"),
    flag("json"),
];

pub(crate) static READY_SPEC: &[OptSpec] = &[
    opt("type"),
    opt("assignee"),
    opt("label"),
    opt("label-any"),
    flag("unlabeled"),
    opt("priority"),
    opt("priority-max"),
    opt("limit"),
    opt("sort"),
    opt("format"),
    flag("json"),
    flag("respect-schedule"),
];

pub(crate) static SEARCH_SPEC: &[OptSpec] = &[
    opt("status"),
    opt("type"),
    opt("assignee"),
    opt("label"),
    opt("label-any"),
    flag("unlabeled"),
    opt("priority"),
    opt("priority-max"),
    opt("limit"),
    opt("sort"),
    opt("format"),
    flag("json"),
];

pub(crate) static UPDATE_SPEC: &[OptSpec] = &[
    opt("status"),
    opt("title"),
    opt("assignee"),
    opt("description"),
    opt("desc"),
    opt("body"),
    opt("type"),
    opt("priority"),
    opt("add-label"),
    opt("remove-label"),
    opt("set-labels"),
    opt("extensions"),
    flag("clear-extensions"),
    flag("json"),
];

pub(crate) static CLOSE_SPEC: &[OptSpec] = &[opt("reason"), flag("json")];

pub(crate) static DEP_ADD_SPEC: &[OptSpec] = &[flag("json")];

pub(crate) static PRIME_SPEC: &[OptSpec] = &[flag("compact"), flag("export"), flag("json")];

pub(crate) static DEDUPE_SPEC: &[OptSpec] = &[flag("write"), flag("json")];

/// The parsed command line: positionals, value options (canonical long
/// name → value, aliases folded), and boolean flags.
#[derive(Debug, Default)]
pub(crate) struct Parsed {
    /// Positional arguments in order.
    pub positionals: Vec<String>,
    /// Value options by canonical long name (last occurrence wins).
    pub options:     HashMap<String, String>,
    /// Boolean flags by long name.
    pub flags:       HashSet<String>,
}

/// Canonicalizes an alias to its primary option name.
fn canonical(long: &str) -> &str {
    match long {
        "desc" | "body" => "description",
        other => other,
    }
}

/// Parses `args` against `spec`.
///
/// # Errors
///
/// Returns the commander-style message for unknown options and missing
/// values.
pub(crate) fn parse(spec: &[OptSpec], args: &[String]) -> Result<Parsed, String> {
    let mut parsed = Parsed::default();
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--" {
            parsed.positionals.extend(args[index + 1..].iter().cloned());
            break;
        }
        if let Some(long) = arg.strip_prefix("--") {
            let (name, inline_value) = match long.split_once('=') {
                Some((name, value)) => (name, Some(value.to_owned())),
                None => (long, None),
            };
            let spec_hit = spec.iter().find(|entry| entry.long == name);
            let Some(entry) = spec_hit else {
                return Err(format!("unknown option '--{name}'"));
            };
            if entry.value {
                let value = if let Some(value) = inline_value {
                    value
                } else {
                    let Some(next) = args.get(index + 1) else {
                        return Err(format!("option '--{name}' requires a value"));
                    };
                    index += 1;
                    next.clone()
                };
                parsed.options.insert(canonical(name).to_owned(), value);
            } else {
                parsed.flags.insert(name.to_owned());
            }
        } else if arg.len() > 1 && arg.starts_with('-') {
            for char in arg[1..].chars() {
                match char {
                    'q' => {
                        parsed.flags.insert("quiet".to_owned());
                    }
                    other => {
                        return Err(format!("unknown option '-{other}'"));
                    }
                }
            }
        } else {
            parsed.positionals.push(arg.clone());
        }
        index += 1;
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(items: &[&str]) -> Vec<String> {
        items.iter().copied().map(str::to_owned).collect()
    }

    #[test]
    fn parses_values_inline_and_detached() {
        let spec = &[opt("title"), opt("limit")];
        let parsed = parse(spec, &argv(&["--title=a b", "--limit", "3", "positional"]))
            .expect("valid input");
        assert_eq!(parsed.options.get("title").map(String::as_str), Some("a b"));
        assert_eq!(parsed.options.get("limit").map(String::as_str), Some("3"));
        assert_eq!(parsed.positionals, vec!["positional"]);
    }

    #[test]
    fn folds_description_aliases() {
        let spec = &[opt("description"), opt("desc"), opt("body")];
        let parsed = parse(spec, &argv(&["--desc", "one"])).expect("valid input");
        assert_eq!(
            parsed.options.get("description").map(String::as_str),
            Some("one")
        );
    }

    #[test]
    fn rejects_unknown_option() {
        let error = parse(&[opt("title")], &argv(&["--bogus"])).expect_err("unknown option");
        assert_eq!(error, "unknown option '--bogus'");
    }

    #[test]
    fn rejects_missing_value() {
        let error = parse(&[opt("title")], &argv(&["--title"])).expect_err("no value");
        assert_eq!(error, "option '--title' requires a value");
    }
}
