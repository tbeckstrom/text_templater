pub mod loader;

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    #[default]
    Text,
    Textarea,
    Number,
    Dropdown,
    Checkbox,
    Multiselect,
    Date,
    /// A read-only value worked out from the other fields by a Tera
    /// expression (`compute = "..."`) rather than typed in.
    Computed,
}

/// One choice in a `dropdown`/`multiselect`. Written either as a bare string
/// (`options = ["Cough", "Fever"]`) or as a table carrying the longer prose
/// it stands for (`options = [{ label = "Irrigated", text = "Irrigated the
/// surgical site with copious sterile saline" }]`). When any option in a
/// field has `text`, the template also gets `<field_key>_text` holding the
/// expansions of what's selected, in declared option order.
#[derive(Debug, Clone)]
pub struct OptionDef {
    pub label: String,
    pub text: Option<String>,
}

impl<'de> Deserialize<'de> for OptionDef {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Label(String),
            Full {
                label: String,
                #[serde(default)]
                text: Option<String>,
            },
        }
        Ok(match Raw::deserialize(deserializer)? {
            Raw::Label(label) => OptionDef { label, text: None },
            Raw::Full { label, text } => OptionDef { label, text },
        })
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
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
    pub options: Vec<OptionDef>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    /// Tera expression; the field is only shown (and only enforced as
    /// `required`) while it evaluates truthy against the current form values.
    #[serde(default)]
    pub visible_if: Option<String>,
    /// For `type = "computed"`: the Tera expression producing this field's
    /// value, e.g. `compute = "dob | age_years"`. Recomputed whenever the
    /// form changes; shown read-only.
    #[serde(default)]
    pub compute: Option<String>,
    /// Heading this field sits under in the form. Consecutive fields naming
    /// the same section are drawn together in one collapsible block, so a
    /// long form can be folded down to the part being worked on.
    #[serde(default)]
    pub section: Option<String>,
}

impl FieldDef {
    /// The declared choices as plain labels — what the form shows and what
    /// the field's value is made of.
    pub fn option_labels(&self) -> impl Iterator<Item = &str> {
        self.options.iter().map(|o| o.label.as_str())
    }

    /// Whether any choice carries expansion prose, i.e. whether this field
    /// should also publish a `<key>_text` to the template.
    pub fn has_option_text(&self) -> bool {
        self.options.iter().any(|o| o.text.is_some())
    }

    /// The expansion for `label`, falling back to the label itself when that
    /// choice has no `text` of its own.
    pub fn text_for(&self, label: &str) -> Option<&str> {
        self.options
            .iter()
            .find(|o| o.label == label)
            .map(|o| o.text.as_deref().unwrap_or(o.label.as_str()))
    }
}

/// A button that fills in several fields at once with a canned set of values,
/// for frequently-used patterns (e.g. "Annual Visit" -> reason + history).
/// Can also (or instead) append one or more pre-filled instances to a
/// repeatable group — see `group` / `group_values`.
#[derive(Debug, Clone, Default, Deserialize)]
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
    /// Tera expression; the button is only shown while it evaluates truthy.
    #[serde(default)]
    pub visible_if: Option<String>,
}

/// A repeatable block of fields (e.g. one "Procedure Step" per tooth worked
/// on) — rendered as an add/remove-able list in the form, and available in
/// `body` as an array of objects: `{% for x in <key> %}{{ x.<field_key> }}`.
///
/// With `source` set, the list is instead driven by a multiselect field: one
/// instance per selected choice, created and removed as the selection
/// changes, with no manual add/remove.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupDef {
    pub key: String,
    pub label: String,
    pub fields: Vec<FieldDef>,
    /// Speed buttons scoped to one instance — they set that instance's own
    /// fields, not the template's top-level ones.
    #[serde(default)]
    pub speed_buttons: Vec<SpeedButtonDef>,
    /// Key of a `multiselect` field whose selection drives this group's
    /// instances (one per selected choice, in declared option order).
    #[serde(default)]
    pub source: Option<String>,
    /// Name the driving choice is exposed under inside each instance
    /// (default `"item"`), e.g. `source_as = "tooth"` → `{{ p.tooth }}`.
    #[serde(default)]
    pub source_as: Option<String>,
    /// Tera expression; the whole group is only shown while it is truthy.
    #[serde(default)]
    pub visible_if: Option<String>,
}

impl GroupDef {
    /// What the driving choice is called inside each instance.
    pub fn source_as_key(&self) -> &str {
        self.source_as.as_deref().unwrap_or("item")
    }
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
    /// Names of field-bearing partials (`partials/<name>.toml`) to pull in —
    /// their fields, groups and speed buttons join this template's form, and
    /// their body becomes available to `{% include "<name>" %}`.
    #[serde(default)]
    pub use_partials: Vec<String>,
    #[serde(default)]
    pub fields: Vec<FieldDef>,
    #[serde(default)]
    pub groups: Vec<GroupDef>,
    #[serde(default)]
    pub speed_buttons: Vec<SpeedButtonDef>,
    pub body: String,
}

/// Shape of a field-bearing partial, `partials/<name>.toml`. Same shape as a
/// template minus the identifying bits — it isn't selectable on its own.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawPartialFile {
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

/// A reusable body section. A plain `partials/<name>.tera` contributes text
/// only; a `partials/<name>.toml` also brings the form controls that text
/// needs, so a whole section (fields and all) can be shared between notes.
#[derive(Debug, Clone, Default)]
pub struct PartialDef {
    pub name: String,
    pub body: String,
    pub fields: Vec<FieldDef>,
    pub groups: Vec<GroupDef>,
    pub speed_buttons: Vec<SpeedButtonDef>,
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
