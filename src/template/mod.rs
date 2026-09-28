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
    /// A multiselect of FDI tooth numbers, picked on an odontogram. Its
    /// options are filled in by the loader (see [`FDI_TEETH`]), so a template
    /// only writes `type = "teeth"`; everywhere else it behaves exactly like a
    /// `multiselect`, including driving a group via `source`.
    Teeth,
}

impl FieldType {
    /// Whether the value is a list of picked options.
    pub fn is_multi(self) -> bool {
        matches!(self, FieldType::Multiselect | FieldType::Teeth)
    }
}

/// Every FDI tooth number a `teeth` field offers, in the order selections are
/// kept (and so read in the note): permanent quadrants 1-4, then primary 5-8.
pub const FDI_TEETH: [&str; 52] = [
    "11", "12", "13", "14", "15", "16", "17", "18", "21", "22", "23", "24", "25", "26", "27", "28",
    "31", "32", "33", "34", "35", "36", "37", "38", "41", "42", "43", "44", "45", "46", "47", "48",
    "51", "52", "53", "54", "55", "61", "62", "63", "64", "65", "71", "72", "73", "74", "75", "81",
    "82", "83", "84", "85",
];

/// Whether an FDI number is a primary tooth (quadrants 5-8).
pub fn is_primary_tooth(fdi: &str) -> bool {
    matches!(fdi.as_bytes().first(), Some(b'5'..=b'8'))
}

/// Each arch in chart order, patient's right to left. Neighbours in one of
/// these lists are adjacent teeth, including across the midline (11 and 21).
const ARCHES: [&[&str]; 4] = [
    &["18", "17", "16", "15", "14", "13", "12", "11", "21", "22", "23", "24", "25", "26", "27", "28"],
    &["48", "47", "46", "45", "44", "43", "42", "41", "31", "32", "33", "34", "35", "36", "37", "38"],
    &["55", "54", "53", "52", "51", "61", "62", "63", "64", "65"],
    &["85", "84", "83", "82", "81", "71", "72", "73", "74", "75"],
];

/// Groups picked teeth into runs of adjacent teeth, each labelled the way a
/// surgeon writes it: `"36"`, `"35-37"` within a quadrant (low to high), or
/// `"12-22"` across the midline (chart order). Spans come out upper arch
/// first, each read from the patient's right.
pub fn contiguous_spans(teeth: &[String]) -> Vec<String> {
    let picked = |t: &str| teeth.iter().any(|p| p == t);
    let mut spans = Vec::new();
    for arch in ARCHES {
        let mut run: Vec<&str> = Vec::new();
        for &tooth in arch.iter().chain(std::iter::once(&"")) {
            if !tooth.is_empty() && picked(tooth) {
                run.push(tooth);
                continue;
            }
            match run.as_slice() {
                [] => {}
                [only] => spans.push(only.to_string()),
                [first, .., last] => {
                    let same_quadrant = first.as_bytes()[0] == last.as_bytes()[0];
                    let (lo, hi) = if same_quadrant && first > last { (last, first) } else { (first, last) };
                    spans.push(format!("{lo}-{hi}"));
                }
            }
            run.clear();
        }
    }
    spans
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
    /// For a `teeth` field: another `teeth` field charted on the same
    /// odontogram. The chart gets a paint mode per field (named by each
    /// field's label), a tooth can be in only one of them, and the linked
    /// field isn't drawn separately.
    #[serde(default)]
    pub linked: Option<String>,
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
    /// Form section to show the button in, beside the fields it fills. A
    /// button without one (or naming a section no field uses) sits in the row
    /// at the top of the form.
    #[serde(default)]
    pub section: Option<String>,
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
    /// Form section to draw the group in, after that section's fields.
    /// Without one (or naming a section no field uses) it is drawn at the end
    /// of the form.
    #[serde(default)]
    pub section: Option<String>,
    /// With a `teeth` source: one instance per run of adjacent teeth (see
    /// [`contiguous_spans`]) instead of one per tooth.
    #[serde(default)]
    pub spans: bool,
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
    /// When set, the partial is also listed as a note of its own under this
    /// name, so a snippet can be used without writing a template around it.
    pub name: Option<String>,
    #[serde(default)]
    pub use_shared: Vec<String>,
    /// Other `.toml` partials whose controls this one brings along, for the
    /// ones it `{% include %}`s.
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
    pub use_partials: Vec<String>,
    /// The `name` it's listed under when selectable on its own.
    pub title: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(teeth: &[&str]) -> Vec<String> {
        contiguous_spans(&teeth.iter().map(|t| t.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn adjacent_teeth_in_a_quadrant_form_one_span_written_low_to_high() {
        assert_eq!(spans(&["35", "36", "37", "46"]), vec!["46", "35-37"]);
        assert_eq!(spans(&["14", "15", "16"]), vec!["14-16"]);
    }

    #[test]
    fn a_span_can_cross_the_midline_and_gaps_split_spans() {
        assert_eq!(spans(&["11", "12", "21", "24"]), vec!["12-21", "24"]);
        assert_eq!(spans(&["36", "38"]), vec!["36", "38"]);
    }
}
