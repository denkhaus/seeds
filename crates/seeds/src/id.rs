//! Validated id newtypes for seeds records.
//!
//! Seed ids are `<project>-<hex4>` where `<hex4>` is exactly four lowercase
//! hex digits; plan ids are `pl-<hex4>` and template ids `tpl-<hex4>`.
//! That shape is the generation-side contract (ids we mint); the READ
//! path accepts any non-empty id — parity with sd 0.5.15, which
//! validates nothing when loading a store (see [`SeedId::read`]).

use std::fmt;
use std::str::FromStr;

use crate::error::IdError;

fn is_hex4(value: &str) -> bool {
    value.len() == 4
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

/// A validated seed id: `<project>-<hex4>`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SeedId(String);

impl SeedId {
    /// Validates and constructs a seed id.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::Seed`] unless the value is `<project>-<hex4>`
    /// with a non-empty project and four lowercase hex digits.
    pub fn try_new(value: &str) -> Result<Self, IdError> {
        // The project part may itself contain '-', so the hex4 suffix is
        // everything after the LAST hyphen.
        match value.rsplit_once('-') {
            Some((project, hex4)) if !project.is_empty() && is_hex4(hex4) => {
                Ok(Self(value.to_owned()))
            }
            _ => Err(IdError::Seed(value.to_owned())),
        }
    }

    /// The id as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Constructs a seed id as read from a store record.
    ///
    /// Read-path parity with the reference (sd 0.5.15): the reference
    /// validates nothing when loading ids — `seeds-fix003`, `nohyphen`,
    /// even `seeds--` all list (probed against the pinned fixture
    /// reference, seeds-3791). Only emptiness is rejected. Ids this
    /// codebase generates are always `<project>-<hex4>` ([`Self::try_new`]
    /// keeps that strict contract for generation-side validation).
    ///
    /// Returns `None` for an empty id.
    #[must_use]
    pub fn read(value: &str) -> Option<Self> {
        (!value.is_empty()).then(|| Self(value.to_owned()))
    }
}

impl fmt::Display for SeedId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for SeedId {
    type Err = IdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::try_new(value)
    }
}

macro_rules! fixed_prefix_id {
    ($(#[$meta:meta])* $name:ident, $prefix:literal, $error:ident, $doc:literal) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            #[doc = concat!("Validates and constructs a ", $doc, " id (`", $prefix, "-<hex4>`).")]
            ///
            /// # Errors
            ///
            /// Returns [`IdError`],
            #[doc = concat!("`::", stringify!($error), "`}, unless the value is `", $prefix, "-<hex4>`.")]
            pub fn try_new(value: &str) -> Result<Self, IdError> {
                let rest = value.strip_prefix(concat!($prefix, "-"));
                if rest.is_some_and(is_hex4) {
                    Ok(Self(value.to_owned()))
                } else {
                    Err(IdError::$error(value.to_owned()))
                }
            }

            /// The id as a string slice.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl FromStr for $name {
            type Err = IdError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::try_new(value)
            }
        }
    };
}

fixed_prefix_id!(
    /// A validated plan id: `pl-<hex4>`.
    PlanId,
    "pl",
    Plan,
    "plan"
);

fixed_prefix_id!(
    /// A validated template id: `tpl-<hex4>`.
    TemplateId,
    "tpl",
    Template,
    "template"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_id_accepts_project_with_hyphens_and_dots() {
        let id = SeedId::try_new("tmp.x-ab12").expect("valid id");
        assert_eq!(id.as_str(), "tmp.x-ab12");
        let id = SeedId::try_new("my-proj-892b").expect("valid id");
        assert_eq!(id.as_str(), "my-proj-892b");
    }

    #[test]
    fn seed_id_rejects_bad_shapes() {
        for bad in [
            "",
            "seeds",
            "seeds-",
            "seeds-AB12",
            "seeds-abc",
            "seeds-abcde",
            "-892b",
        ] {
            assert_eq!(
                SeedId::try_new(bad),
                Err(IdError::Seed(bad.to_owned())),
                "{bad}"
            );
        }
    }

    #[test]
    fn read_path_accepts_any_non_empty_id() {
        // sd 0.5.15 validates nothing on load (probed against the
        // fixture reference): the reader must agree on every record.
        for shape in ["seeds-fix003", "nohyphen", "seeds--", "seeds-a1b2c3", "x"] {
            let id = SeedId::read(shape).expect("any non-empty id loads");
            assert_eq!(id.as_str(), shape);
        }
        assert_eq!(SeedId::read(""), None, "only emptiness is rejected");
    }

    #[test]
    fn plan_and_template_ids_validate_prefix_and_hex() {
        assert!(PlanId::try_new("pl-3ad6").is_ok());
        assert_eq!(
            PlanId::try_new("seeds-3ad6"),
            Err(IdError::Plan("seeds-3ad6".to_owned()))
        );
        assert!(TemplateId::try_new("tpl-5643").is_ok());
        assert_eq!(
            TemplateId::try_new("pl-5643"),
            Err(IdError::Template("pl-5643".to_owned()))
        );
        assert!(PlanId::try_new("pl-3ad63").is_err());
        assert!(TemplateId::try_new("tpl-XYZW").is_err());
    }
}
