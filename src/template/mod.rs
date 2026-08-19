pub mod loader;

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Text,
    Textarea,
    Number,
    Dropdown,
    Checkbox,
    Multiselect,
    Date,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldDef {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: FieldType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<toml::Value>,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
}

/// A button that fills in several fields at once with a canned set of values,
/// for frequently-used patterns (e.g. "Annual Visit" -> reason + history).
/// Can also (or instead) append one or more pre-filled instances to a
/// repeatable group — see `group` / `group_values`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeedButtonDef {
    pub label: String,
    #[serde(default)]
    pub values: toml::Table,
    /// Key of a `[[groups]]` this button adds instances to.
    #[serde(default)]
    pub group: Option<String>,
    /// One table per instance to append to `group` when clicked, each in the
    /// same shape as that group's fields' `default`s.
    #[serde(default)]
    pub group_values: Vec<toml::Table>,
}

/// A repeatable block of fields (e.g. one "Procedure Step" per tooth worked
/// on) — rendered as an add/remove-able list in the form, and available in
/// `body` as an array of objects: `{% for x in <key> %}{{ x.<field_key> }}`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupDef {
    pub key: String,
    pub label: String,
    pub fields: Vec<FieldDef>,
}

/// Shape of a template `.toml` file as written on disk.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawTemplateFile {
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub use_shared: Vec<String>,
    #[serde(default)]
    pub fields: Vec<FieldDef>,
    #[serde(default)]
    pub groups: Vec<GroupDef>,
    #[serde(default)]
    pub speed_buttons: Vec<SpeedButtonDef>,
    pub body: String,
}

/// Shape of the reserved `shared_fields.toml` file.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedFieldsFile {
    #[serde(default)]
    pub fields: Vec<FieldDef>,
}

/// A fully resolved template: shared fields already merged in.
#[derive(Debug, Clone)]
pub struct TemplateDef {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub fields: Vec<FieldDef>,
    pub groups: Vec<GroupDef>,
    pub speed_buttons: Vec<SpeedButtonDef>,
    pub body: String,
}
