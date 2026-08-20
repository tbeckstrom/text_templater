use std::error::Error as _;

use tera::{Context, Tera};

use crate::field::{form_state_to_context_json, FormState, GroupsState};
use crate::template::{FieldDef, GroupDef, PartialDef};

/// The name the current template's body is registered under, so it can call
/// `{% include "some_partial" %}` and have Tera resolve it against the other
/// named templates (the partials) also registered in the same instance.
const BODY_TEMPLATE_NAME: &str = "__note_body__";

/// Builds the Tera context a template body (and any `visible_if` expression)
/// sees: every top-level field by key, `<key>_text` expansions for option
/// fields that define them, and each group's instances as an array of objects
/// under the group's key (`{% for x in <key> %}{{ x.<field> }}`).
pub fn build_context(
    fields: &[FieldDef],
    state: &FormState,
    groups: &[GroupDef],
    groups_state: &GroupsState,
) -> Context {
    let mut ctx = Context::new();
    if let serde_json::Value::Object(map) = form_state_to_context_json(fields, state) {
        for (key, value) in map {
            ctx.insert(key, &value);
        }
    }
    for group in groups {
        let instances = groups_state.get(&group.key).map(Vec::as_slice).unwrap_or(&[]);
        let json_instances: Vec<_> = instances
            .iter()
            .map(|inst| instance_context_json(group, inst))
            .collect();
        ctx.insert(group.key.clone(), &json_instances);
    }
    ctx
}

/// One group instance as the template sees it: its own fields (plus any
/// `_text` expansions), and — for a `source`-driven group — the choice it
/// stands for, under the group's `source_as` name.
pub fn instance_context_json(
    group: &GroupDef,
    instance: &crate::field::GroupInstance,
) -> serde_json::Value {
    let mut obj = form_state_to_context_json(&group.fields, &instance.values);
    if let (Some(item), serde_json::Value::Object(map)) = (&instance.source_item, &mut obj) {
        map.insert(
            group.source_as_key().to_string(),
            serde_json::Value::String(item.clone()),
        );
    }
    obj
}

/// Renders `body` against `ctx`, with `partials` available to `{% include %}`.
/// A fresh [`Tera`] instance is built per call — note bodies are short, so
/// this is cheap enough to do on every frame.
pub fn render_body_with_context(
    body: &str,
    partials: &[PartialDef],
    ctx: &Context,
) -> Result<String, String> {
    let mut tera = Tera::default();
    for partial in partials {
        tera.add_raw_template(&partial.name, &partial.body)
            .map_err(|e| format_tera_error(&e))?;
    }
    tera.add_raw_template(BODY_TEMPLATE_NAME, body)
        .map_err(|e| format_tera_error(&e))?;
    tera.render(BODY_TEMPLATE_NAME, ctx)
        .map_err(|e| format_tera_error(&e))
}

/// Convenience wrapper that builds the context and renders in one step.
#[cfg(test)]
pub fn render_body(
    body: &str,
    partials: &[PartialDef],
    fields: &[FieldDef],
    state: &FormState,
    groups: &[GroupDef],
    groups_state: &GroupsState,
) -> Result<String, String> {
    let ctx = build_context(fields, state, groups, groups_state);
    render_body_with_context(body, partials, &ctx)
}

fn format_tera_error(err: &tera::Error) -> String {
    let mut msg = err.to_string();
    let mut source = err.source();
    while let Some(s) = source {
        msg.push_str(&format!("\ncaused by: {s}"));
        source = s.source();
    }
    msg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::{add_group_instance, build_groups_state, FieldValue};
    use crate::template::{FieldDef, PartialDef};

    fn no_groups() -> (Vec<GroupDef>, GroupsState) {
        (Vec::new(), GroupsState::new())
    }

    #[test]
    fn conditional_and_loop_render_correctly() {
        let mut state = FormState::new();
        state.insert(
            "symptoms".to_string(),
            FieldValue::MultiSelect(vec!["Cough".to_string(), "Fever".to_string()]),
        );
        state.insert("follow_up_needed".to_string(), FieldValue::Bool(true));

        let body = "{% for s in symptoms %}{{ s }};{% endfor %}{% if follow_up_needed %}FOLLOWUP{% else %}NONE{% endif %}";
        let (groups, groups_state) = no_groups();
        let rendered = render_body(body, &[], &[], &state, &groups, &groups_state).unwrap();
        assert_eq!(rendered, "Cough;Fever;FOLLOWUP");
    }

    #[test]
    fn false_branch_and_empty_loop() {
        let mut state = FormState::new();
        state.insert("symptoms".to_string(), FieldValue::MultiSelect(vec![]));
        state.insert("follow_up_needed".to_string(), FieldValue::Bool(false));

        let body = "{% for s in symptoms %}{{ s }};{% endfor %}{% if follow_up_needed %}FOLLOWUP{% else %}NONE{% endif %}";
        let (groups, groups_state) = no_groups();
        let rendered = render_body(body, &[], &[], &state, &groups, &groups_state).unwrap();
        assert_eq!(rendered, "NONE");
    }

    #[test]
    fn bad_template_syntax_reports_error_instead_of_panicking() {
        let state = FormState::new();
        let (groups, groups_state) = no_groups();
        let result = render_body("{% if unclosed %}", &[], &[], &state, &groups, &groups_state);
        assert!(result.is_err());
    }

    #[test]
    fn body_can_include_a_partial() {
        let mut state = FormState::new();
        state.insert("name".to_string(), FieldValue::Text("Alex".to_string()));

        let partials = vec![PartialDef {
            name: "header".to_string(),
            body: "Hello, {{ name }}!".to_string(),
            ..Default::default()
        }];
        let (groups, groups_state) = no_groups();
        let rendered = render_body(
            "{% include \"header\" %} Welcome.",
            &partials,
            &[],
            &state,
            &groups,
            &groups_state,
        )
        .unwrap();
        assert_eq!(rendered, "Hello, Alex! Welcome.");
    }

    #[test]
    fn missing_partial_reports_error_instead_of_panicking() {
        let state = FormState::new();
        let (groups, groups_state) = no_groups();
        let result = render_body(
            "{% include \"does_not_exist\" %}",
            &[],
            &[],
            &state,
            &groups,
            &groups_state,
        );
        assert!(result.is_err());
    }

    #[test]
    fn body_can_loop_over_a_repeatable_group() {
        let state = FormState::new();
        let groups = vec![GroupDef {
            key: "procedures".to_string(),
            label: "Procedure Step".to_string(),
            fields: vec![FieldDef {
                key: "tooth".to_string(),
                label: "Tooth".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        }];
        let mut groups_state = build_groups_state(&groups);
        add_group_instance(&mut groups_state, &groups[0]);
        add_group_instance(&mut groups_state, &groups[0]);
        groups_state.get_mut("procedures").unwrap()[0]
            .values
            .insert("tooth".to_string(), FieldValue::Text("14".to_string()));
        groups_state.get_mut("procedures").unwrap()[1]
            .values
            .insert("tooth".to_string(), FieldValue::Text("15".to_string()));

        let body = "{% for p in procedures %}{{ loop.index }}:{{ p.tooth }};{% endfor %}";
        let rendered = render_body(body, &[], &[], &state, &groups, &groups_state).unwrap();
        assert_eq!(rendered, "1:14;2:15;");
    }

    #[test]
    fn empty_group_renders_as_empty_loop() {
        let state = FormState::new();
        let groups = vec![GroupDef {
            key: "procedures".to_string(),
            label: "Procedure Step".to_string(),
            ..Default::default()
        }];
        let groups_state = build_groups_state(&groups);

        let body = "{% if procedures %}has steps{% else %}no steps{% endif %}";
        let rendered = render_body(body, &[], &[], &state, &groups, &groups_state).unwrap();
        assert_eq!(rendered, "no steps");
    }
}
