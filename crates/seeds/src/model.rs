//! The seeds data model: map-backed records with typed accessors.
//!
//! Every record keeps its raw [`serde_json::Map`] (with `preserve_order`)
//! as the source of truth for serialization. Known fields are exposed via
//! typed getters/setters; every other field is an unknown field that MUST
//! survive read-modify-write untouched — additive fields are the format's
//! only sanctioned extension mechanism (README compat contract).

use serde_json::Value;

use crate::error::RecordError;
use crate::id::{PlanId, SeedId, TemplateId};

/// The ordered field map backing every record.
pub type Fields = serde_json::Map<String, Value>;

/// Issue lifecycle status, as written by sd 0.5.15.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    /// `open`
    Open,
    /// `in_progress`
    InProgress,
    /// `closed`
    Closed,
}

impl Status {
    /// The wire representation.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Closed => "closed",
        }
    }

    /// Parses the wire representation, `None` for anything else.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "open" => Some(Self::Open),
            "in_progress" => Some(Self::InProgress),
            "closed" => Some(Self::Closed),
            _ => None,
        }
    }
}

/// Issue type, as written by sd 0.5.15.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SeedType {
    /// `task`
    Task,
    /// `bug`
    Bug,
    /// `feature`
    Feature,
    /// `epic`
    Epic,
}

impl SeedType {
    /// The wire representation.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Bug => "bug",
            Self::Feature => "feature",
            Self::Epic => "epic",
        }
    }

    /// Parses the wire representation, `None` for anything else.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "task" => Some(Self::Task),
            "bug" => Some(Self::Bug),
            "feature" => Some(Self::Feature),
            "epic" => Some(Self::Epic),
            _ => None,
        }
    }
}

/// Issue priority, 0 (P0, most urgent) through 4.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Priority(u8);

impl Priority {
    /// The highest urgency.
    pub const P0: Self = Self(0);

    /// Validates a priority number.
    ///
    /// # Errors
    ///
    /// Returns a [`RecordError::Field`] for values outside `0..=4`.
    pub fn try_new(value: u8) -> Result<Self, RecordError> {
        if value <= 4 {
            Ok(Self(value))
        } else {
            Err(RecordError::Field {
                field:   "priority",
                message: format!("expected 0-4, got {value}"),
            })
        }
    }

    /// The numeric priority.
    #[must_use]
    pub fn get(self) -> u8 {
        self.0
    }
}

/// Reads a string field, `None` when absent or null.
fn str_field<'a>(fields: &'a Fields, name: &str) -> Option<&'a str> {
    fields.get(name).and_then(Value::as_str)
}

/// Reads a string-array field (`blockedBy`, `blocks`, ...).
fn str_array_field<'a>(fields: &'a Fields, name: &str) -> Vec<&'a str> {
    fields
        .get(name)
        .and_then(Value::as_array)
        .map(|entries| entries.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

/// A seed issue record from `issues.jsonl`.
///
/// The raw field map is preserved verbatim on write, including unknown
/// fields and their original ordering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeedRecord {
    id:     SeedId,
    fields: Fields,
}

impl SeedRecord {
    /// Builds a record from a parsed JSONL object, validating the known
    /// fields.
    ///
    /// # Errors
    ///
    /// Returns a [`RecordError`] when `id` is missing or malformed, when
    /// `title` is missing, or when `status`/`type`/`priority` carry a
    /// value of the wrong shape.
    pub fn try_from_fields(fields: Fields) -> Result<Self, RecordError> {
        let id_text = str_field(&fields, "id").ok_or(RecordError::Field {
            field:   "id",
            message: "missing or not a string".to_owned(),
        })?;
        let id = SeedId::try_new(id_text)?;

        str_field(&fields, "title").ok_or_else(|| RecordError::Field {
            field:   "title",
            message: "missing or not a string".to_owned(),
        })?;

        Self::check_known_fields(&fields)?;
        Ok(Self { id, fields })
    }

    fn check_known_fields(fields: &Fields) -> Result<(), RecordError> {
        if let Some(status) = str_field(fields, "status")
            && Status::parse(status).is_none()
        {
            return Err(RecordError::Field {
                field:   "status",
                message: format!("unknown status `{status}`"),
            });
        }
        if let Some(kind) = str_field(fields, "type")
            && SeedType::parse(kind).is_none()
        {
            return Err(RecordError::Field {
                field:   "type",
                message: format!("unknown type `{kind}`"),
            });
        }
        if let Some(priority) = fields.get("priority") {
            let number = priority.as_u64().ok_or_else(|| RecordError::Field {
                field:   "priority",
                message: format!("not a number: {priority}"),
            })?;
            u8::try_from(number)
                .map_err(|_| RecordError::Field {
                    field:   "priority",
                    message: format!("out of range: {number}"),
                })
                .and_then(Priority::try_new)?;
        }
        Ok(())
    }

    /// The validated id.
    #[must_use]
    pub fn id(&self) -> &SeedId {
        &self.id
    }

    /// The title (validated present at construction).
    #[must_use]
    pub fn title(&self) -> &str {
        str_field(&self.fields, "title").unwrap_or_default()
    }

    /// The description, if present.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        str_field(&self.fields, "description")
    }

    /// The status; sd always writes it, hand-written records may omit it.
    #[must_use]
    pub fn status(&self) -> Option<Status> {
        str_field(&self.fields, "status").and_then(Status::parse)
    }

    /// The issue type.
    #[must_use]
    pub fn seed_type(&self) -> Option<SeedType> {
        str_field(&self.fields, "type").and_then(SeedType::parse)
    }

    /// The priority.
    #[must_use]
    pub fn priority(&self) -> Option<Priority> {
        self.fields
            .get("priority")
            .and_then(Value::as_u64)
            .and_then(|number| u8::try_from(number).ok())
            .and_then(|number| Priority::try_new(number).ok())
    }

    /// The labels.
    #[must_use]
    pub fn labels(&self) -> Vec<&str> {
        str_array_field(&self.fields, "labels")
    }

    /// The assignee, if present.
    #[must_use]
    pub fn assignee(&self) -> Option<&str> {
        str_field(&self.fields, "assignee")
    }

    /// The `blockedBy` dependency ids.
    #[must_use]
    pub fn blocked_by(&self) -> Vec<&str> {
        str_array_field(&self.fields, "blockedBy")
    }

    /// The inverse-dependency (`blocks`) ids.
    #[must_use]
    pub fn blocks(&self) -> Vec<&str> {
        str_array_field(&self.fields, "blocks")
    }

    /// The `createdAt` timestamp, if present.
    #[must_use]
    pub fn created_at(&self) -> Option<&str> {
        str_field(&self.fields, "createdAt")
    }

    /// The `updatedAt` timestamp, if present.
    #[must_use]
    pub fn updated_at(&self) -> Option<&str> {
        str_field(&self.fields, "updatedAt")
    }

    /// Replaces the title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.set_field("title", Value::String(title.into()));
    }

    /// Sets the description; `None` removes the field (sd omits it).
    pub fn set_description(&mut self, description: Option<&str>) {
        match description {
            Some(text) => self.set_field("description", Value::String(text.to_owned())),
            None => {
                self.fields.remove("description");
            }
        }
    }

    /// Sets the status.
    pub fn set_status(&mut self, status: Status) {
        self.set_field("status", Value::String(status.as_str().to_owned()));
    }

    /// Sets the issue type.
    pub fn set_seed_type(&mut self, seed_type: SeedType) {
        self.set_field("type", Value::String(seed_type.as_str().to_owned()));
    }

    /// Sets the priority.
    ///
    /// # Errors
    ///
    /// Returns the [`Priority::try_new`] error for out-of-range values.
    pub fn set_priority(&mut self, priority: u8) -> Result<(), RecordError> {
        let priority = Priority::try_new(priority)?;
        self.set_field("priority", Value::from(u64::from(priority.get())));
        Ok(())
    }

    /// Sets the labels.
    pub fn set_labels<I, S>(&mut self, labels: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let labels: Vec<Value> = labels
            .into_iter()
            .map(|label| Value::String(label.into()))
            .collect();
        self.set_field("labels", Value::Array(labels));
    }

    /// Sets the assignee; `None` removes the field.
    pub fn set_assignee(&mut self, assignee: Option<&str>) {
        match assignee {
            Some(name) => self.set_field("assignee", Value::String(name.to_owned())),
            None => {
                self.fields.remove("assignee");
            }
        }
    }

    /// Appends a `blockedBy` dependency id (deduplicated).
    pub fn add_blocked_by(&mut self, dependency: &str) {
        let mut deps = self.blocked_by();
        if !deps.contains(&dependency) {
            deps.push(dependency);
        }
        let ids: Vec<Value> = deps
            .into_iter()
            .map(|id| Value::String(id.to_owned()))
            .collect();
        self.set_field("blockedBy", Value::Array(ids));
    }

    /// Removes a field entirely (sd omits empty labels, cleared
    /// extensions, and similar instead of writing empty values).
    pub fn remove_field(&mut self, name: &str) {
        self.fields.remove(name);
    }

    /// Reads any field (known or unknown) by name.
    #[must_use]
    pub fn field(&self, name: &str) -> Option<&Value> {
        self.fields.get(name)
    }

    /// Writes any field by name. Updating an existing key keeps its
    /// position; a new key is appended, matching sd's insertion order.
    pub fn set_field(&mut self, name: &str, value: Value) {
        self.fields.insert(name.to_owned(), value);
    }

    /// The raw field map.
    #[must_use]
    pub fn fields(&self) -> &Fields {
        &self.fields
    }

    /// Serializes the record as one compact JSONL line, exactly as sd
    /// writes it.
    #[must_use]
    pub fn to_json_line(&self) -> String {
        serde_json::to_string(&self.fields).expect("a JSON map always serializes")
    }
}

macro_rules! opaque_record {
    ($(#[$meta:meta])* $name:ident, $id:ty, $id_error:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name {
            id: $id,
            fields: Fields,
        }

        impl $name {
            /// Builds a record from a parsed JSONL object, validating the id.
            ///
            /// # Errors
            ///
            /// Returns a [`RecordError`] when `id` is missing or malformed.
            pub fn try_from_fields(fields: Fields) -> Result<Self, RecordError> {
                let id_text = str_field(&fields, "id").ok_or(RecordError::Field {
                    field: "id",
                    message: "missing or not a string".to_owned(),
                })?;
                let id = <$id>::try_new(id_text)?;
                Ok(Self { id, fields })
            }

            /// The validated id.
            #[must_use]
            pub fn id(&self) -> &$id {
                &self.id
            }

            /// Reads any field (known or unknown) by name.
            #[must_use]
            pub fn field(&self, name: &str) -> Option<&Value> {
                self.fields.get(name)
            }

            /// Writes any field by name, preserving position like sd does.
            pub fn set_field(&mut self, name: &str, value: Value) {
                self.fields.insert(name.to_owned(), value);
            }

            /// The raw field map.
            #[must_use]
            pub fn fields(&self) -> &Fields {
                &self.fields
            }

            /// Serializes the record as one compact JSONL line.
            #[must_use]
            pub fn to_json_line(&self) -> String {
                serde_json::to_string(&self.fields).expect("a JSON map always serializes")
            }

            /// The `seed` this record refers to, if present.
            #[must_use]
            pub fn seed(&self) -> Option<&str> {
                str_field(&self.fields, "seed")
            }

            /// The record name, if present.
            #[must_use]
            pub fn name(&self) -> Option<&str> {
                str_field(&self.fields, "name")
            }

            /// The `createdAt` timestamp, if present.
            #[must_use]
            pub fn created_at(&self) -> Option<&str> {
                str_field(&self.fields, "createdAt")
            }

            /// The `updatedAt` timestamp, if present.
            #[must_use]
            pub fn updated_at(&self) -> Option<&str> {
                str_field(&self.fields, "updatedAt")
            }
        }
    };
}

opaque_record!(
    /// A plan record from `plans.jsonl` (`pl-<hex4>`). Sections, status,
    /// and children stay schema-opaque here: the format reserves them for
    /// sd's plan machinery, and this crate only needs read/write fidelity.
    PlanRecord,
    PlanId,
    Plan
);

opaque_record!(
    /// A template record from `templates.jsonl` (`tpl-<hex4>`). Steps stay
    /// schema-opaque: this crate only needs read/write fidelity.
    TemplateRecord,
    TemplateId,
    Template
);

impl PlanRecord {
    /// The plan revision counter, if present.
    #[must_use]
    pub fn revision(&self) -> Option<u64> {
        self.fields.get("revision").and_then(Value::as_u64)
    }

    /// The adopted child seed ids.
    #[must_use]
    pub fn children(&self) -> Vec<&str> {
        str_array_field(&self.fields, "children")
    }

    /// The plan sections, if present.
    #[must_use]
    pub fn sections(&self) -> Option<&Value> {
        self.fields.get("sections")
    }
}

impl TemplateRecord {
    /// The template steps, if present.
    #[must_use]
    pub fn steps(&self) -> Option<&Vec<Value>> {
        self.fields.get("steps").and_then(Value::as_array)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn sample_fields() -> Fields {
        let mut fields = Fields::new();
        fields.insert("id".to_owned(), json!("seeds-e218"));
        fields.insert("title".to_owned(), json!("Format core"));
        fields.insert("status".to_owned(), json!("in_progress"));
        fields.insert("type".to_owned(), json!("task"));
        fields.insert("priority".to_owned(), json!(1));
        fields.insert("labels".to_owned(), json!(["ready-for-agent", "loop"]));
        fields.insert("assignee".to_owned(), json!("fabro"));
        fields.insert("blockedBy".to_owned(), json!(["seeds-0000"]));
        fields
    }

    #[test]
    fn record_exposes_typed_model() {
        let record = SeedRecord::try_from_fields(sample_fields()).expect("valid record");
        assert_eq!(record.id().as_str(), "seeds-e218");
        assert_eq!(record.title(), "Format core");
        assert_eq!(record.status(), Some(Status::InProgress));
        assert_eq!(record.seed_type(), Some(SeedType::Task));
        assert_eq!(record.priority().map(Priority::get), Some(1));
        assert_eq!(record.labels(), vec!["ready-for-agent", "loop"]);
        assert_eq!(record.assignee(), Some("fabro"));
        assert_eq!(record.blocked_by(), vec!["seeds-0000"]);
        assert!(record.description().is_none());
    }

    #[test]
    fn record_rejects_bad_known_fields() {
        let mut fields = sample_fields();
        fields.insert("status".to_owned(), json!("frozen"));
        assert!(SeedRecord::try_from_fields(fields).is_err());

        let mut fields = sample_fields();
        fields.insert("priority".to_owned(), json!(9));
        assert!(SeedRecord::try_from_fields(fields).is_err());

        let mut fields = sample_fields();
        fields.remove("title");
        assert!(SeedRecord::try_from_fields(fields).is_err());
    }

    #[test]
    fn setters_keep_field_position() {
        let mut record = SeedRecord::try_from_fields(sample_fields()).expect("valid record");
        record.set_status(Status::Closed);
        assert_eq!(record.status(), Some(Status::Closed));

        let line = record.to_json_line();
        let keys: Vec<&str> = record.fields().keys().map(String::as_str).collect();
        // Position preserved: status stays third, a new field appends.
        assert_eq!(keys[..3], ["id", "title", "status"]);
        record.set_field("x_new", json!(true));
        let keys: Vec<&str> = record.fields().keys().map(String::as_str).collect();
        assert_eq!(keys.last().copied(), Some("x_new"));
        assert!(line.contains("\"status\":\"closed\""));
    }

    #[test]
    fn priority_bounds() {
        assert_eq!(Priority::try_new(4).map(Priority::get), Ok(4));
        assert!(Priority::try_new(5).is_err());
    }
}
