use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Map, Value};

use crate::template::{FieldDef, FieldType, GroupDef, SpeedButtonDef};

/// A date field's value: the raw text the user typed or picked, plus the
/// parsed result. Kept separate so a field can hold text like "2026-13-40"
/// mid-edit without losing what was typed, while still knowing it's invalid.
#[derive(Debug, Clone)]
pub struct DateInput {
    pub text: String,
    pub parsed: Option<jiff::civil::Date>,
}

impl DateInput {
    fn from_date(date: jiff::civil::Date) -> Self {
        Self {
            text: date.to_string(),
            parsed: Some(date),
        }
    }

    fn from_text(text: String) -> Self {
        let parsed = text.trim().parse().ok();
        Self { text, parsed }
    }

    pub fn is_invalid(&self) -> bool {
        !self.text.trim().is_empty() && self.parsed.is_none()
    }
}

#[derive(Debug, Clone)]
pub enum FieldValue {
    Text(String),
    Number(f64),
    Bool(bool),
    Date(DateInput),
    MultiSelect(Vec<String>),
}

impl FieldValue {
    pub fn default_for(field: &FieldDef) -> Self {
        field
            .default
            .as_ref()
            .and_then(|v| FieldValue::from_toml_value(field.field_type, v))
            .unwrap_or_else(|| FieldValue::empty_for(field.field_type))
    }

    /// The "nothing entered yet" value for a field type, used when there's no
    /// `default` (or an unusable one) to fall back to.
    fn empty_for(field_type: FieldType) -> Self {
        match field_type {
            FieldType::Text | FieldType::Textarea | FieldType::Dropdown => {
                FieldValue::Text(String::new())
            }
            FieldType::Number => FieldValue::Number(0.0),
            FieldType::Checkbox => FieldValue::Bool(false),
            FieldType::Date => FieldValue::Date(DateInput::from_date(today())),
            FieldType::Multiselect => FieldValue::MultiSelect(Vec::new()),
        }
    }

    /// Interprets a `toml::Value` (a field's `default = ...`, or one entry of
    /// a speed button's `values = { ... }`) as this field type's value.
    /// `None` if the TOML shape doesn't match (e.g. a string where a number
    /// was expected) rather than guessing.
    fn from_toml_value(field_type: FieldType, value: &toml::Value) -> Option<Self> {
        match field_type {
            FieldType::Text | FieldType::Textarea | FieldType::Dropdown => {
                value.as_str().map(|s| FieldValue::Text(s.to_owned()))
            }
            FieldType::Number => value
                .as_float()
                .or_else(|| value.as_integer().map(|i| i as f64))
                .map(FieldValue::Number),
            FieldType::Checkbox => value.as_bool().map(FieldValue::Bool),
            FieldType::Date => value.as_str().map(|s| {
                if s == "today" {
                    FieldValue::Date(DateInput::from_date(today()))
                } else {
                    FieldValue::Date(DateInput::from_text(s.to_owned()))
                }
            }),
            FieldType::Multiselect => value.as_array().map(|arr| {
                FieldValue::MultiSelect(
                    arr.iter()
                        .filter_map(|v| v.as_str().map(str::to_owned))
                        .collect(),
                )
            }),
        }
    }

    /// Rebuild a field's value from a previously-saved JSON value (history entry or
    /// draft), falling back to `None` if the shape doesn't match this field's type
    /// (e.g. the template's field type changed since it was saved).
    fn from_json(field: &FieldDef, value: &Value) -> Option<Self> {
        match field.field_type {
            FieldType::Text | FieldType::Textarea | FieldType::Dropdown => {
                value.as_str().map(|s| FieldValue::Text(s.to_string()))
            }
            FieldType::Number => value.as_f64().map(FieldValue::Number),
            FieldType::Checkbox => value.as_bool().map(FieldValue::Bool),
            FieldType::Date => value
                .as_str()
                .map(|s| FieldValue::Date(DateInput::from_text(s.to_string()))),
            FieldType::Multiselect => value.as_array().map(|arr| {
                FieldValue::MultiSelect(
                    arr.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect(),
                )
            }),
        }
    }

    fn to_json(&self) -> Value {
        match self {
            FieldValue::Text(s) => Value::String(s.clone()),
            FieldValue::Number(n) => Value::from(*n),
            FieldValue::Bool(b) => Value::Bool(*b),
            FieldValue::Date(d) => Value::String(d.text.clone()),
            FieldValue::MultiSelect(v) => {
                Value::Array(v.iter().map(|s| Value::String(s.clone())).collect())
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            FieldValue::Text(s) => s.trim().is_empty(),
            FieldValue::MultiSelect(v) => v.is_empty(),
            FieldValue::Date(d) => d.parsed.is_none(),
            FieldValue::Number(_) | FieldValue::Bool(_) => false,
        }
    }
}

pub fn today() -> jiff::civil::Date {
    jiff::Zoned::now().date()
}

pub type FormState = HashMap<String, FieldValue>;

pub fn build_form_state(fields: &[FieldDef]) -> FormState {
    fields
        .iter()
        .map(|f| (f.key.clone(), FieldValue::default_for(f)))
        .collect()
}

/// One repetition of a [`GroupDef`] (e.g. one "Procedure Step"). `id` is a
/// process-lifetime-unique handle used to scope egui widget ids per instance,
/// stable across reordering/removal of *other* instances in the same list.
#[derive(Debug, Clone)]
pub struct GroupInstance {
    pub id: u64,
    pub values: FormState,
}

static NEXT_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);

fn next_instance_id() -> u64 {
    NEXT_INSTANCE_ID.fetch_add(1, Ordering::Relaxed)
}

fn new_instance(fields: &[FieldDef]) -> GroupInstance {
    GroupInstance {
        id: next_instance_id(),
        values: build_form_state(fields),
    }
}

/// group key -> its current list of instances.
pub type GroupsState = HashMap<String, Vec<GroupInstance>>;

pub fn build_groups_state(groups: &[GroupDef]) -> GroupsState {
    groups.iter().map(|g| (g.key.clone(), Vec::new())).collect()
}

/// Appends one fresh, default-valued instance of `group` to `groups_state`
/// (e.g. the "+ Add Procedure Step" button).
pub fn add_group_instance(groups_state: &mut GroupsState, group: &GroupDef) {
    groups_state
        .entry(group.key.clone())
        .or_default()
        .push(new_instance(&group.fields));
}

/// Applies a speed button: overwrites any top-level fields named in
/// `sb.values`, then (if `sb.group` names one of `groups`) appends one new
/// instance per table in `sb.group_values`. Unmatched keys (typos, a field
/// that's since been removed) and type mismatches are silently skipped
/// rather than treated as errors — a speed button always does what it can.
pub fn apply_speed_button(
    sb: &SpeedButtonDef,
    fields: &[FieldDef],
    state: &mut FormState,
    groups: &[GroupDef],
    groups_state: &mut GroupsState,
) {
    for field in fields {
        if let Some(value) = sb.values.get(&field.key)
            && let Some(field_value) = FieldValue::from_toml_value(field.field_type, value)
        {
            state.insert(field.key.clone(), field_value);
        }
    }

    let Some(group_key) = &sb.group else { return };
    let Some(group) = groups.iter().find(|g| &g.key == group_key) else {
        return;
    };
    let instances = groups_state.entry(group.key.clone()).or_default();
    for values in &sb.group_values {
        let mut instance = new_instance(&group.fields);
        for field in &group.fields {
            if let Some(value) = values.get(&field.key)
                && let Some(field_value) = FieldValue::from_toml_value(field.field_type, value)
            {
                instance.values.insert(field.key.clone(), field_value);
            }
        }
        instances.push(instance);
    }
}

/// Serializes the fields' current values (in template field order) to a JSON
/// object. Used both for a template's top-level fields and, per-instance, for
/// a group's fields.
pub fn form_state_to_json(fields: &[FieldDef], state: &FormState) -> Value {
    let mut map = Map::new();
    for field in fields {
        if let Some(value) = state.get(&field.key) {
            map.insert(field.key.clone(), value.to_json());
        }
    }
    Value::Object(map)
}

/// Rebuilds a form state for `fields`, starting from defaults and overlaying
/// any values found in `json` (as produced by [`form_state_to_json`]). Missing
/// or mismatched keys just keep their default.
pub fn form_state_from_json(fields: &[FieldDef], json: &Value) -> FormState {
    let mut state = build_form_state(fields);
    if let Some(obj) = json.as_object() {
        for field in fields {
            if let Some(value) = obj.get(&field.key)
                && let Some(field_value) = FieldValue::from_json(field, value)
            {
                state.insert(field.key.clone(), field_value);
            }
        }
    }
    state
}

/// Reserved key under which a snapshot's repeatable-group instances are
/// nested, alongside the top-level fields. Chosen so it can't collide with a
/// real field `key` (which is a bare TOML identifier) and so history/draft
/// entries saved before groups existed — plain flat objects — still parse
/// fine (they just have no `__groups__`, i.e. no saved group instances).
const GROUPS_JSON_KEY: &str = "__groups__";

/// Serializes a template's whole form (top-level fields + every repeatable
/// group's instances) to one JSON value, for a history entry or draft.
pub fn snapshot_to_json(
    fields: &[FieldDef],
    state: &FormState,
    groups: &[GroupDef],
    groups_state: &GroupsState,
) -> Value {
    let Value::Object(mut map) = form_state_to_json(fields, state) else {
        unreachable!("form_state_to_json always returns an object")
    };
    if !groups.is_empty() {
        let mut groups_json = Map::new();
        for group in groups {
            let instances = groups_state.get(&group.key).map(Vec::as_slice).unwrap_or(&[]);
            let json_instances = instances
                .iter()
                .map(|inst| form_state_to_json(&group.fields, &inst.values))
                .collect();
            groups_json.insert(group.key.clone(), Value::Array(json_instances));
        }
        map.insert(GROUPS_JSON_KEY.to_string(), Value::Object(groups_json));
    }
    Value::Object(map)
}

/// Inverse of [`snapshot_to_json`]: rebuilds both the top-level form state and
/// every group's instances from a saved snapshot, defaulting whatever isn't
/// found or doesn't match the current template shape.
pub fn snapshot_from_json(
    fields: &[FieldDef],
    groups: &[GroupDef],
    json: &Value,
) -> (FormState, GroupsState) {
    let state = form_state_from_json(fields, json);
    let mut groups_state = build_groups_state(groups);

    if let Some(groups_json) = json.as_object().and_then(|obj| obj.get(GROUPS_JSON_KEY)) {
        for group in groups {
            let Some(saved_instances) = groups_json.get(&group.key).and_then(|v| v.as_array())
            else {
                continue;
            };
            let instances = saved_instances
                .iter()
                .map(|inst_json| GroupInstance {
                    id: next_instance_id(),
                    values: form_state_from_json(&group.fields, inst_json),
                })
                .collect();
            groups_state.insert(group.key.clone(), instances);
        }
    }

    (state, groups_state)
}

/// Whether `json` (a saved history entry's or draft's field values) has enough
/// overlap with `fields` to be worth restoring: a non-empty object with at
/// least one key that still matches a current field of a compatible type.
///
/// Entries saved before this feature existed store `Value::Null` and
/// correctly report `false` here; entries saved for a template whose fields
/// were since edited (key renamed/removed, type changed) also fall back to
/// `false` once none of their keys line up with the current field set.
///
/// Only checks top-level fields — a template whose groups changed shape still
/// restores, with each instance's mismatched fields just falling back to
/// defaults the same way a plain field would.
pub fn is_restorable(fields: &[FieldDef], json: &Value) -> bool {
    let Some(obj) = json.as_object() else {
        return false;
    };
    fields.iter().any(|f| {
        obj.get(&f.key)
            .is_some_and(|v| FieldValue::from_json(f, v).is_some())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date_field(key: &str) -> FieldDef {
        FieldDef {
            key: key.to_string(),
            label: key.to_string(),
            field_type: FieldType::Date,
            required: false,
            default: None,
            options: vec![],
            min: None,
            max: None,
        }
    }

    fn text_field(key: &str, required: bool) -> FieldDef {
        FieldDef {
            key: key.to_string(),
            label: key.to_string(),
            field_type: FieldType::Text,
            required,
            default: None,
            options: vec![],
            min: None,
            max: None,
        }
    }

    fn multiselect_field(key: &str) -> FieldDef {
        FieldDef {
            key: key.to_string(),
            label: key.to_string(),
            field_type: FieldType::Multiselect,
            required: false,
            default: None,
            options: vec!["A".into(), "B".into()],
            min: None,
            max: None,
        }
    }

    #[test]
    fn valid_and_invalid_date_text_is_detected() {
        let valid = DateInput::from_text("2026-08-19".to_string());
        assert!(!valid.is_invalid());
        assert_eq!(valid.parsed.unwrap().to_string(), "2026-08-19");

        let invalid = DateInput::from_text("not-a-date".to_string());
        assert!(invalid.is_invalid());
        assert!(invalid.parsed.is_none());

        let blank = DateInput::from_text(String::new());
        assert!(!blank.is_invalid(), "blank text isn't 'invalid', just empty");
        assert!(blank.parsed.is_none());
    }

    #[test]
    fn form_state_round_trips_through_json() {
        let fields = vec![
            text_field("name", true),
            date_field("dob"),
            multiselect_field("tags"),
        ];
        let mut state = build_form_state(&fields);
        state.insert("name".into(), FieldValue::Text("Alex".into()));
        state.insert(
            "dob".into(),
            FieldValue::Date(DateInput::from_text("2026-01-05".into())),
        );
        state.insert(
            "tags".into(),
            FieldValue::MultiSelect(vec!["A".into()]),
        );

        let json = form_state_to_json(&fields, &state);
        let restored = form_state_from_json(&fields, &json);

        match restored.get("name").unwrap() {
            FieldValue::Text(s) => assert_eq!(s, "Alex"),
            other => panic!("expected Text, got {other:?}"),
        }
        match restored.get("dob").unwrap() {
            FieldValue::Date(d) => assert_eq!(d.text, "2026-01-05"),
            other => panic!("expected Date, got {other:?}"),
        }
        match restored.get("tags").unwrap() {
            FieldValue::MultiSelect(v) => assert_eq!(v, &vec!["A".to_string()]),
            other => panic!("expected MultiSelect, got {other:?}"),
        }
    }

    #[test]
    fn missing_keys_in_json_fall_back_to_defaults() {
        let fields = vec![text_field("name", false)];
        let restored = form_state_from_json(&fields, &Value::Object(Map::new()));
        match restored.get("name").unwrap() {
            FieldValue::Text(s) => assert_eq!(s, ""),
            other => panic!("expected empty Text default, got {other:?}"),
        }
    }

    #[test]
    fn is_restorable_rejects_legacy_null_entries() {
        let fields = vec![text_field("name", false)];
        assert!(!is_restorable(&fields, &Value::Null));
    }

    #[test]
    fn is_restorable_rejects_empty_object() {
        let fields = vec![text_field("name", false)];
        assert!(!is_restorable(&fields, &Value::Object(Map::new())));
    }

    #[test]
    fn is_restorable_rejects_object_with_no_matching_keys() {
        let fields = vec![text_field("name", false)];
        let json = serde_json::json!({"unrelated_key": "value"});
        assert!(!is_restorable(&fields, &json));
    }

    #[test]
    fn is_restorable_rejects_type_mismatch_even_with_matching_key() {
        // Simulates a field whose type changed (e.g. text -> checkbox) since
        // the entry was saved: the key matches but the stored shape doesn't.
        let fields = vec![date_field("dob")];
        let json = serde_json::json!({"dob": true});
        assert!(!is_restorable(&fields, &json));
    }

    #[test]
    fn is_restorable_accepts_object_with_at_least_one_matching_field() {
        let fields = vec![text_field("name", false), date_field("dob")];
        let json = serde_json::json!({"name": "Alex", "some_removed_field": "x"});
        assert!(is_restorable(&fields, &json));
    }

    fn checkbox_field(key: &str) -> FieldDef {
        FieldDef {
            key: key.to_string(),
            label: key.to_string(),
            field_type: FieldType::Checkbox,
            required: false,
            default: None,
            options: vec![],
            min: None,
            max: None,
        }
    }

    fn toml_table(pairs: &[(&str, toml::Value)]) -> toml::Table {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    fn speed_button(values: toml::Table) -> SpeedButtonDef {
        SpeedButtonDef {
            label: "Test".to_string(),
            values,
            group: None,
            group_values: vec![],
        }
    }

    fn no_groups() -> (Vec<GroupDef>, GroupsState) {
        (Vec::new(), GroupsState::new())
    }

    #[test]
    fn speed_button_overwrites_matching_fields() {
        let fields = vec![
            text_field("reason", false),
            text_field("history", false),
            checkbox_field("labs_ordered"),
        ];
        let mut state = build_form_state(&fields);
        state.insert("reason".into(), FieldValue::Text("stale".into()));

        let sb = speed_button(toml_table(&[
            ("reason", toml::Value::String("Annual check up".into())),
            ("history", toml::Value::String("none".into())),
            ("labs_ordered", toml::Value::Boolean(true)),
        ]));
        let (groups, mut groups_state) = no_groups();
        apply_speed_button(&sb, &fields, &mut state, &groups, &mut groups_state);

        match state.get("reason").unwrap() {
            FieldValue::Text(s) => assert_eq!(s, "Annual check up"),
            other => panic!("expected Text, got {other:?}"),
        }
        match state.get("history").unwrap() {
            FieldValue::Text(s) => assert_eq!(s, "none"),
            other => panic!("expected Text, got {other:?}"),
        }
        match state.get("labs_ordered").unwrap() {
            FieldValue::Bool(b) => assert!(*b),
            other => panic!("expected Bool, got {other:?}"),
        }
    }

    #[test]
    fn speed_button_ignores_unknown_and_mismatched_keys() {
        let fields = vec![text_field("reason", false)];
        let mut state = build_form_state(&fields);

        let sb = speed_button(toml_table(&[
            ("reason", toml::Value::Boolean(true)), // wrong type for a text field
            ("nonexistent_field", toml::Value::String("x".into())),
        ]));
        let (groups, mut groups_state) = no_groups();
        apply_speed_button(&sb, &fields, &mut state, &groups, &mut groups_state);

        match state.get("reason").unwrap() {
            FieldValue::Text(s) => assert_eq!(s, "", "mismatched type should be left untouched"),
            other => panic!("expected Text, got {other:?}"),
        }
    }

    #[test]
    fn speed_button_can_set_a_multiselect_field() {
        let fields = vec![multiselect_field("symptoms")];
        let mut state = build_form_state(&fields);

        let sb = speed_button(toml_table(&[(
            "symptoms",
            toml::Value::Array(vec![toml::Value::String("Fever".into())]),
        )]));
        let (groups, mut groups_state) = no_groups();
        apply_speed_button(&sb, &fields, &mut state, &groups, &mut groups_state);

        match state.get("symptoms").unwrap() {
            FieldValue::MultiSelect(v) => assert_eq!(v, &vec!["Fever".to_string()]),
            other => panic!("expected MultiSelect, got {other:?}"),
        }
    }

    fn procedure_group() -> GroupDef {
        GroupDef {
            key: "procedures".to_string(),
            label: "Procedure Step".to_string(),
            fields: vec![text_field("tooth", true), text_field("procedure_type", false)],
        }
    }

    #[test]
    fn speed_button_appends_a_group_instance() {
        let fields: Vec<FieldDef> = vec![];
        let mut state = build_form_state(&fields);
        let groups = vec![procedure_group()];
        let mut groups_state = build_groups_state(&groups);

        let mut sb = speed_button(toml::Table::new());
        sb.group = Some("procedures".to_string());
        sb.group_values = vec![toml_table(&[
            ("tooth", toml::Value::String("14".into())),
            ("procedure_type", toml::Value::String("Extraction".into())),
        ])];
        apply_speed_button(&sb, &fields, &mut state, &groups, &mut groups_state);

        let instances = groups_state.get("procedures").unwrap();
        assert_eq!(instances.len(), 1);
        match instances[0].values.get("tooth").unwrap() {
            FieldValue::Text(s) => assert_eq!(s, "14"),
            other => panic!("expected Text, got {other:?}"),
        }
    }

    #[test]
    fn speed_button_can_append_multiple_group_instances_at_once() {
        let fields: Vec<FieldDef> = vec![];
        let mut state = build_form_state(&fields);
        let groups = vec![procedure_group()];
        let mut groups_state = build_groups_state(&groups);

        let mut sb = speed_button(toml::Table::new());
        sb.group = Some("procedures".to_string());
        sb.group_values = vec![
            toml_table(&[("tooth", toml::Value::String("1".into()))]),
            toml_table(&[("tooth", toml::Value::String("2".into()))]),
        ];
        apply_speed_button(&sb, &fields, &mut state, &groups, &mut groups_state);

        assert_eq!(groups_state.get("procedures").unwrap().len(), 2);
    }

    #[test]
    fn speed_button_appending_to_existing_instances_does_not_clear_them() {
        let fields: Vec<FieldDef> = vec![];
        let mut state = build_form_state(&fields);
        let groups = vec![procedure_group()];
        let mut groups_state = build_groups_state(&groups);
        groups_state
            .get_mut("procedures")
            .unwrap()
            .push(new_instance(&groups[0].fields));

        let mut sb = speed_button(toml::Table::new());
        sb.group = Some("procedures".to_string());
        sb.group_values = vec![toml_table(&[("tooth", toml::Value::String("3".into()))])];
        apply_speed_button(&sb, &fields, &mut state, &groups, &mut groups_state);

        assert_eq!(groups_state.get("procedures").unwrap().len(), 2);
    }

    #[test]
    fn group_instance_ids_are_unique() {
        let fields = vec![text_field("tooth", false)];
        let a = new_instance(&fields);
        let b = new_instance(&fields);
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn snapshot_round_trips_top_level_fields_and_group_instances() {
        let fields = vec![text_field("patient_name", true)];
        let groups = vec![procedure_group()];

        let mut state = build_form_state(&fields);
        state.insert("patient_name".into(), FieldValue::Text("Alex".into()));

        let mut groups_state = build_groups_state(&groups);
        let mut inst = new_instance(&groups[0].fields);
        inst.values
            .insert("tooth".into(), FieldValue::Text("14".into()));
        groups_state.get_mut("procedures").unwrap().push(inst);

        let json = snapshot_to_json(&fields, &state, &groups, &groups_state);
        let (restored_state, restored_groups) = snapshot_from_json(&fields, &groups, &json);

        match restored_state.get("patient_name").unwrap() {
            FieldValue::Text(s) => assert_eq!(s, "Alex"),
            other => panic!("expected Text, got {other:?}"),
        }
        let restored_instances = restored_groups.get("procedures").unwrap();
        assert_eq!(restored_instances.len(), 1);
        match restored_instances[0].values.get("tooth").unwrap() {
            FieldValue::Text(s) => assert_eq!(s, "14"),
            other => panic!("expected Text, got {other:?}"),
        }
    }

    #[test]
    fn snapshot_from_json_on_legacy_flat_entry_yields_no_group_instances() {
        // A history/draft entry saved before groups existed is just a flat
        // object with no `__groups__` key at all.
        let fields = vec![text_field("patient_name", true)];
        let groups = vec![procedure_group()];
        let json = serde_json::json!({"patient_name": "Alex"});

        let (state, groups_state) = snapshot_from_json(&fields, &groups, &json);

        match state.get("patient_name").unwrap() {
            FieldValue::Text(s) => assert_eq!(s, "Alex"),
            other => panic!("expected Text, got {other:?}"),
        }
        assert!(groups_state.get("procedures").unwrap().is_empty());
    }

    #[test]
    fn snapshot_to_json_omits_groups_key_when_template_has_no_groups() {
        let fields = vec![text_field("patient_name", true)];
        let state = build_form_state(&fields);
        let json = snapshot_to_json(&fields, &state, &[], &GroupsState::new());
        assert!(json.as_object().unwrap().get(GROUPS_JSON_KEY).is_none());
    }
}
