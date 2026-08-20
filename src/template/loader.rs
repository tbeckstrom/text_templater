use std::fs;
use std::path::Path;

use super::{
    FieldDef, GroupDef, PartialDef, RawPartialFile, RawTemplateFile, SharedFieldsFile,
    SpeedButtonDef, TemplateDef,
};

pub struct LoadResult {
    pub templates: Vec<TemplateDef>,
    /// Every reusable body section in `partials/`, includable from any
    /// template's `body` via `{% include "name" %}`. Ones defined as `.toml`
    /// additionally carry form controls, pulled in via `use_partials`.
    pub partials: Vec<PartialDef>,
    /// (file name, error message) for any template or partial that failed to load.
    pub errors: Vec<(String, String)>,
}

pub fn load_templates(dir: &Path) -> LoadResult {
    let mut errors = Vec::new();

    let shared_fields = load_shared_fields(dir, &mut errors);
    let partials = load_partials(dir, &shared_fields, &mut errors);

    let mut paths: Vec<_> = match fs::read_dir(dir) {
        Ok(entries) => entries.filter_map(|e| e.ok()).map(|e| e.path()).collect(),
        Err(e) => {
            errors.push((dir.display().to_string(), e.to_string()));
            Vec::new()
        }
    };
    paths.sort();

    let mut templates = Vec::new();
    for path in paths {
        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            continue;
        }
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if file_name == "shared_fields.toml" {
            continue;
        }

        match load_one(&path, &shared_fields, &partials) {
            Ok(template) => templates.push(template),
            Err(e) => errors.push((file_name, e)),
        }
    }

    templates.sort_by(|a, b| a.name.cmp(&b.name));

    LoadResult {
        templates,
        partials,
        errors,
    }
}

/// Loads the reserved `partials/` subdirectory. A `*.tera` file is a
/// text-only section; a `*.toml` file is a field-bearing one (same shape as a
/// template, minus id/name) whose controls a template opts into with
/// `use_partials`. A missing directory just means no partials; a file that
/// fails to parse is reported without stopping the rest.
fn load_partials(
    dir: &Path,
    shared_fields: &[FieldDef],
    errors: &mut Vec<(String, String)>,
) -> Vec<PartialDef> {
    let partials_dir = dir.join("partials");
    if !partials_dir.exists() {
        return Vec::new();
    }

    let mut paths: Vec<_> = match fs::read_dir(&partials_dir) {
        Ok(entries) => entries.filter_map(|e| e.ok()).map(|e| e.path()).collect(),
        Err(e) => {
            errors.push(("partials/".to_string(), e.to_string()));
            Vec::new()
        }
    };
    paths.sort();

    let mut partials: Vec<PartialDef> = Vec::new();
    for path in paths {
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if extension != "tera" && extension != "toml" {
            continue;
        }
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let file_label = format!("partials/{name}.{extension}");

        if partials.iter().any(|p| p.name == name) {
            errors.push((
                file_label,
                format!("a partial named '{name}' is already defined"),
            ));
            continue;
        }

        let contents = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                errors.push((file_label, e.to_string()));
                continue;
            }
        };

        if extension == "tera" {
            partials.push(PartialDef {
                name,
                body: contents,
                ..Default::default()
            });
            continue;
        }

        match toml::from_str::<RawPartialFile>(&contents) {
            Ok(raw) => match resolve_shared(&raw.use_shared, shared_fields) {
                Ok(mut fields) => {
                    fields.extend(raw.fields);
                    partials.push(PartialDef {
                        name,
                        body: raw.body,
                        fields,
                        groups: raw.groups,
                        speed_buttons: raw.speed_buttons,
                    });
                }
                Err(e) => errors.push((file_label, e)),
            },
            Err(e) => errors.push((file_label, e.to_string())),
        }
    }
    partials
}

/// Looks up `keys` in `shared_fields`, preserving the order asked for.
fn resolve_shared(keys: &[String], shared_fields: &[FieldDef]) -> Result<Vec<FieldDef>, String> {
    keys.iter()
        .map(|key| {
            shared_fields
                .iter()
                .find(|f| &f.key == key)
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "use_shared references unknown key '{key}' (not found in shared_fields.toml)"
                    )
                })
        })
        .collect()
}

/// Rejects a merged form that would carry the same field or group key twice —
/// otherwise one silently shadows the other in the Tera context.
fn check_for_duplicates(fields: &[FieldDef], groups: &[GroupDef]) -> Result<(), String> {
    let mut seen = std::collections::HashSet::new();
    for field in fields {
        if !seen.insert(&field.key) {
            return Err(format!("duplicate field key '{}'", field.key));
        }
    }
    let mut seen_groups = std::collections::HashSet::new();
    for group in groups {
        if !seen_groups.insert(&group.key) {
            return Err(format!("duplicate group key '{}'", group.key));
        }
        if seen.contains(&group.key) {
            return Err(format!(
                "group key '{}' collides with a field of the same name",
                group.key
            ));
        }
    }
    Ok(())
}

fn load_shared_fields(dir: &Path, errors: &mut Vec<(String, String)>) -> Vec<FieldDef> {
    let path = dir.join("shared_fields.toml");
    if !path.exists() {
        return Vec::new();
    }
    let result = fs::read_to_string(&path)
        .map_err(|e| e.to_string())
        .and_then(|text| toml::from_str::<SharedFieldsFile>(&text).map_err(|e| e.to_string()));
    match result {
        Ok(file) => file.fields,
        Err(e) => {
            errors.push(("shared_fields.toml".to_string(), e));
            Vec::new()
        }
    }
}

fn load_one(
    path: &Path,
    shared_fields: &[FieldDef],
    partials: &[PartialDef],
) -> Result<TemplateDef, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let raw: RawTemplateFile = toml::from_str(&text).map_err(|e| e.to_string())?;

    let mut fields = resolve_shared(&raw.use_shared, shared_fields)?;
    fields.extend(raw.fields);
    let mut groups: Vec<GroupDef> = raw.groups;
    let mut speed_buttons: Vec<SpeedButtonDef> = raw.speed_buttons;

    // Field-bearing partials contribute their controls after the template's
    // own, so a template's fields stay at the top of the form.
    for name in &raw.use_partials {
        let partial = partials.iter().find(|p| &p.name == name).ok_or_else(|| {
            format!("use_partials references unknown partial '{name}' (expected partials/{name}.toml)")
        })?;
        fields.extend(partial.fields.iter().cloned());
        groups.extend(partial.groups.iter().cloned());
        speed_buttons.extend(partial.speed_buttons.iter().cloned());
    }

    check_for_duplicates(&fields, &groups)?;

    let id = raw.id.unwrap_or_else(|| {
        path.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default()
    });

    Ok(TemplateDef {
        id,
        name: raw.name,
        description: raw.description,
        fields,
        groups,
        speed_buttons,
        body: raw.body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, contents: &str) {
        fs::write(dir.join(name), contents).unwrap();
    }

    #[test]
    fn merges_shared_fields_before_own_fields_in_declared_order() {
        let dir = std::env::temp_dir().join(format!("note_templater_test_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();

        write(
            &dir,
            "shared_fields.toml",
            r#"
            [[fields]]
            key = "patient_name"
            label = "Patient Name"
            type = "text"
            "#,
        );
        write(
            &dir,
            "visit.toml",
            r#"
            name = "Visit"
            use_shared = ["patient_name"]
            body = "{{ patient_name }}: {{ notes }}"

            [[fields]]
            key = "notes"
            label = "Notes"
            type = "textarea"
            "#,
        );

        let result = load_templates(&dir);
        fs::remove_dir_all(&dir).ok();

        assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
        assert_eq!(result.templates.len(), 1);
        let keys: Vec<_> = result.templates[0].fields.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(keys, vec!["patient_name", "notes"]);
    }

    #[test]
    fn unknown_use_shared_key_is_reported_as_an_error() {
        let dir = std::env::temp_dir().join(format!("note_templater_test_{}", std::process::id() + 1));
        fs::create_dir_all(&dir).unwrap();

        write(
            &dir,
            "visit.toml",
            r#"
            name = "Visit"
            use_shared = ["does_not_exist"]
            body = "hi"
            "#,
        );

        let result = load_templates(&dir);
        fs::remove_dir_all(&dir).ok();

        assert!(result.templates.is_empty());
        assert_eq!(result.errors.len(), 1);
    }

    fn write_partial(dir: &Path, name: &str, contents: &str) {
        let p = dir.join("partials");
        fs::create_dir_all(&p).unwrap();
        fs::write(p.join(name), contents).unwrap();
    }

    fn scratch_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "note_templater_partials_{}_{}",
            std::process::id(),
            tag
        ));
        fs::remove_dir_all(&dir).ok();
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn field_bearing_partial_contributes_fields_groups_and_speed_buttons() {
        let dir = scratch_dir("contributes");
        write_partial(
            &dir,
            "blocks.toml",
            r#"
            body = "{% for b in blocks %}{{ b.tooth }};{% endfor %}"

            [[fields]]
            key = "block_note"
            label = "Block note"
            type = "text"

            [[speed_buttons]]
            label = "Preset"
            values = { block_note = "preset" }

            [[groups]]
            key = "blocks"
            label = "Block"

            [[groups.fields]]
            key = "tooth"
            label = "Tooth"
            type = "text"
            "#,
        );
        write(
            &dir,
            "note.toml",
            r#"
            name = "Note"
            use_partials = ["blocks"]
            body = "X: {% include \"blocks\" %}"

            [[fields]]
            key = "own"
            label = "Own"
            type = "text"
            "#,
        );

        let result = load_templates(&dir);
        fs::remove_dir_all(&dir).ok();
        assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);

        let t = &result.templates[0];
        // The template's own field stays first; the partial's are appended.
        let keys: Vec<_> = t.fields.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(keys, vec!["own", "block_note"]);
        assert_eq!(t.groups.len(), 1);
        assert_eq!(t.groups[0].key, "blocks");
        assert_eq!(t.groups[0].fields.len(), 1);
        assert_eq!(t.speed_buttons.len(), 1);
        assert_eq!(t.speed_buttons[0].label, "Preset");
    }

    #[test]
    fn a_partial_can_be_shared_by_two_templates() {
        let dir = scratch_dir("shared");
        write_partial(
            &dir,
            "blocks.toml",
            r#"
            body = "{% for b in blocks %}{{ b.tooth }};{% endfor %}"

            [[groups]]
            key = "blocks"
            label = "Block"

            [[groups.fields]]
            key = "tooth"
            label = "Tooth"
            type = "text"
            "#,
        );
        for name in ["a.toml", "b.toml"] {
            write(
                &dir,
                name,
                r#"
                name = "T"
                use_partials = ["blocks"]
                body = "{% include \"blocks\" %}"
                "#,
            );
        }

        let result = load_templates(&dir);
        fs::remove_dir_all(&dir).ok();
        assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
        assert_eq!(result.templates.len(), 2);
        for t in &result.templates {
            assert_eq!(t.groups.len(), 1, "each template gets its own copy");
        }
    }

    #[test]
    fn a_template_that_does_not_opt_in_gets_no_partial_fields() {
        let dir = scratch_dir("nooptin");
        write_partial(
            &dir,
            "blocks.toml",
            r#"
            body = "x"

            [[fields]]
            key = "block_note"
            label = "Block note"
            type = "text"
            "#,
        );
        write(&dir, "note.toml", "name = \"Note\"\nbody = \"hi\"\n");

        let result = load_templates(&dir);
        fs::remove_dir_all(&dir).ok();
        assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
        assert!(result.templates[0].fields.is_empty());
    }

    #[test]
    fn unknown_use_partials_name_is_reported() {
        let dir = scratch_dir("unknown");
        write(
            &dir,
            "note.toml",
            r#"
            name = "Note"
            use_partials = ["nope"]
            body = "hi"
            "#,
        );
        let result = load_templates(&dir);
        fs::remove_dir_all(&dir).ok();
        assert!(result.templates.is_empty());
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].1.contains("nope"));
    }

    #[test]
    fn a_key_colliding_with_the_partial_is_reported_not_silently_shadowed() {
        let dir = scratch_dir("collide");
        write_partial(
            &dir,
            "blocks.toml",
            r#"
            body = "x"

            [[fields]]
            key = "shared_key"
            label = "From partial"
            type = "text"
            "#,
        );
        write(
            &dir,
            "note.toml",
            r#"
            name = "Note"
            use_partials = ["blocks"]
            body = "hi"

            [[fields]]
            key = "shared_key"
            label = "From template"
            type = "text"
            "#,
        );

        let result = load_templates(&dir);
        fs::remove_dir_all(&dir).ok();
        assert!(result.templates.is_empty());
        assert_eq!(result.errors.len(), 1);
        assert!(
            result.errors[0].1.contains("shared_key"),
            "the error should name the clashing key, got: {}",
            result.errors[0].1
        );
    }

    #[test]
    fn a_broken_partial_is_reported_without_stopping_other_templates() {
        let dir = scratch_dir("broken");
        write_partial(&dir, "bad.toml", "this is not = valid toml [[[");
        write(&dir, "note.toml", "name = \"Note\"\nbody = \"still fine\"\n");

        let result = load_templates(&dir);
        fs::remove_dir_all(&dir).ok();
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.templates.len(), 1, "the good template still loads");
    }

    #[test]
    fn a_partial_can_pull_in_shared_fields() {
        let dir = scratch_dir("partialshared");
        write(
            &dir,
            "shared_fields.toml",
            r#"
            [[fields]]
            key = "patient_name"
            label = "Patient Name"
            type = "text"
            "#,
        );
        write_partial(
            &dir,
            "blocks.toml",
            r#"
            use_shared = ["patient_name"]
            body = "{{ patient_name }}"
            "#,
        );
        write(
            &dir,
            "note.toml",
            r#"
            name = "Note"
            use_partials = ["blocks"]
            body = "{% include \"blocks\" %}"
            "#,
        );

        let result = load_templates(&dir);
        fs::remove_dir_all(&dir).ok();
        assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
        let keys: Vec<_> = result.templates[0].fields.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(keys, vec!["patient_name"]);
    }

    /// Regression guard: the bundled examples in `examples/templates/` are what
    /// `storage::seed_examples_if_empty` copies into a fresh install, so they must
    /// always load cleanly (e.g. `body` must stay before any `[[fields]]` block —
    /// TOML would otherwise silently attach it to the last field table instead of
    /// the document root).
    #[test]
    fn bundled_example_templates_load_without_errors() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/templates");
        let result = load_templates(&dir);
        assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);
        assert_eq!(result.templates.len(), 4);
        assert!(
            result.partials.iter().any(|p| p.name == "header"),
            "expected the bundled `partials/header.tera` to load"
        );
        for template in &result.templates {
            assert!(!template.body.is_empty());
            assert!(!template.fields.is_empty());
        }
    }

    /// Regression guard for the `{% include "header" %}` composition: with
    /// default field values, both bundled templates must actually render
    /// (not just parse as TOML) — this is what would catch e.g. a partial
    /// name typo or the header partial failing to load.
    #[test]
    fn bundled_example_templates_render_with_default_values() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/templates");
        let result = load_templates(&dir);
        assert!(result.errors.is_empty(), "unexpected errors: {:?}", result.errors);

        for template in &result.templates {
            let state = crate::field::build_form_state(&template.fields);
            let groups_state = crate::field::build_groups_state(&template.groups);
            let rendered = crate::render::render_body(
                &template.body,
                &result.partials,
                &template.fields,
                &state,
                &template.groups,
                &groups_state,
            );
            assert!(
                rendered.is_ok(),
                "template '{}' failed to render: {:?}",
                template.id,
                rendered
            );
            // Only the templates that actually pull in the shared header
            // should be expected to show its content.
            if template.body.contains(r#"{% include "header" %}"#) {
                assert!(
                    rendered.unwrap().contains("Patient:"),
                    "expected the included header partial's content in '{}'",
                    template.id
                );
            }
        }
    }

    /// Every field in the OMS note should sit under a section heading, and
    /// the sections should stay contiguous — the form folds by consecutive
    /// runs, so a field declared out of order would split its own heading in
    /// two.
    #[test]
    fn oms_note_fields_are_grouped_into_contiguous_sections() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/templates");
        let result = load_templates(&dir);
        let t = result
            .templates
            .iter()
            .find(|t| t.id == "oms_procedure_note")
            .unwrap();

        assert!(
            t.fields.iter().all(|f| f.section.is_some()),
            "every field should declare a section"
        );

        let mut seen: Vec<&str> = Vec::new();
        for f in &t.fields {
            let section = f.section.as_deref().unwrap();
            if seen.last() != Some(&section) {
                assert!(
                    !seen.contains(&section),
                    "section {section:?} appears in two separate runs"
                );
                seen.push(section);
            }
        }
        assert_eq!(
            seen,
            vec![
                "Patient",
                "Diagnosis",
                "Procedure",
                "Anesthesia",
                "Vitals",
                "History & Indications",
                "Description of Procedure",
            ]
        );
    }

    /// The bundled OMS note's computed tally must actually evaluate against
    /// real field values — a `compute` expression that silently yields blank
    /// would look like a working read-only field.
    #[test]
    fn oms_computed_total_local_anesthetic_evaluates() {
        use crate::field::*;
        use crate::visibility::{recompute_fields, ExprEvaluator};

        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/templates");
        let result = load_templates(&dir);
        let t = result
            .templates
            .iter()
            .find(|t| t.id == "oms_procedure_note")
            .unwrap();

        let mut state = build_form_state(&t.fields);
        state.insert(
            "local_used".into(),
            FieldValue::MultiSelect(vec![
                "Lidocaine 2% w/ epi".into(),
                "Bupivacaine 0.5% w/ epi".into(),
            ]),
        );
        state.insert("lidocaine_carps".into(), FieldValue::Number(2.0));
        state.insert("bupivacaine_carps".into(), FieldValue::Number(1.0));

        let groups_state = build_groups_state(&t.groups);
        let mut ctx = crate::render::build_context(&t.fields, &state, &t.groups, &groups_state);
        let mut evaluator = ExprEvaluator::default();
        recompute_fields(&t.fields, &mut state, &mut ctx, &mut evaluator);

        // 3 carpules selected x 1.7 mL; articaine/mepivacaine aren't selected
        // so their counts must not be included.
        match state.get("total_local_ml").unwrap() {
            FieldValue::Text(s) => assert!(
                s.starts_with("5.1"),
                "expected 3 x 1.7 = 5.1 mL, got {s:?}"
            ),
            other => panic!("expected Text, got {other:?}"),
        }
    }

    /// The blanks a template author leaves for manual completion (`# __ to
    /// # __`, `The ___ root`, the `# ***` block header) have to reach the
    /// rendered note intact — they're the cue to fill something in, so
    /// losing them to formatting markup would be silent data loss.
    #[test]
    fn fill_in_blanks_survive_into_the_rendered_note() {
        use crate::field::*;
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/templates");
        let result = load_templates(&dir);
        let t = result
            .templates
            .iter()
            .find(|t| t.id == "oms_procedure_note")
            .unwrap();
        let blocks = t.groups.iter().find(|g| g.key == "blocks").unwrap();

        let state = build_form_state(&t.fields);
        let mut groups_state = build_groups_state(&t.groups);
        add_group_instance(&mut groups_state, blocks);
        let inst = groups_state.get_mut("blocks").unwrap().last_mut().unwrap();
        inst.values.insert(
            "soft_tissue".into(),
            FieldValue::MultiSelect(vec!["Flap from tooth to tooth".into()]),
        );
        inst.values.insert(
            "ostectomy_sectioning".into(),
            FieldValue::MultiSelect(vec!["Root section".into()]),
        );

        let note = crate::render::render_body(
            &t.body,
            &result.partials,
            &t.fields,
            &state,
            &t.groups,
            &groups_state,
        )
        .unwrap();
        let plain = crate::formatting::to_plain_text(&crate::formatting::parse(&note));

        assert!(
            plain.contains("from # __ to # __"),
            "the tooth-to-tooth blanks were altered: {plain}"
        );
        assert!(
            plain.contains("The ___ root was sectioned"),
            "the root blank was altered: {plain}"
        );
        assert!(
            plain.contains("# ***"),
            "the default block header blank was altered: {plain}"
        );
    }

    /// End-to-end guard for the OMS procedure note — the TextBlaze phrase this
    /// app was built to replace. Exercises, in one render: a selection-driven
    /// group (one diagnosis row per tooth picked), per-instance speed buttons
    /// filling a procedure block, option `text` expansions in declared order,
    /// the `[[each]]`/`[[s]]` singular-plural swap, whole-number formatting,
    /// and "a, b and c" joining.
    #[test]
    fn oms_procedure_note_renders_a_realistic_case() {
        use crate::field::*;
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/templates");
        let result = load_templates(&dir);
        let t = result
            .templates
            .iter()
            .find(|t| t.id == "oms_procedure_note")
            .expect("oms_procedure_note.toml should be a bundled example");

        let mut state = build_form_state(&t.fields);
        for (k, v) in [
            ("pt_age", FieldValue::Number(24.0)),
            ("pt_sex", FieldValue::Text("female".into())),
            ("pmh", FieldValue::Text("asthma\nanxiety".into())),
            (
                "diagnosis_teeth",
                FieldValue::MultiSelect(vec!["1".into(), "17".into()]),
            ),
            ("ext", FieldValue::Bool(true)),
            ("ext_teeth", FieldValue::Text("1, 17".into())),
            ("ivga", FieldValue::Bool(true)),
            ("ga_time", FieldValue::Number(45.0)),
            (
                "ga_meds",
                FieldValue::MultiSelect(vec![
                    "midazolam".into(),
                    "ketamine".into(),
                    "propofol".into(),
                ]),
            ),
            ("lidocaine_carps", FieldValue::Number(2.0)),
        ] {
            state.insert(k.into(), v);
        }

        let mut groups_state = build_groups_state(&t.groups);
        for g in &t.groups {
            sync_source_group(g, &state, &mut groups_state);
        }
        for inst in groups_state.get_mut("tooth_diagnoses").unwrap() {
            inst.values.insert(
                "diagnosis".into(),
                FieldValue::MultiSelect(vec!["symptomatic".into(), "PBI".into()]),
            );
        }

        let blocks = t.groups.iter().find(|g| g.key == "blocks").unwrap();
        for (header, button, multiple) in [
            ("#1, #16", "Mx 3rd Molar", true),
            ("#17", "Md 3rd Molar, vertical section", false),
        ] {
            add_group_instance(&mut groups_state, blocks);
            let last = groups_state.get_mut("blocks").unwrap().last_mut().unwrap();
            let sb = blocks
                .speed_buttons
                .iter()
                .find(|s| s.label == button)
                .unwrap();
            apply_values(&blocks.fields, &sb.values, &mut last.values);
            last.values
                .insert("section_header".into(), FieldValue::Text(header.into()));
            last.values
                .insert("multiple_teeth".into(), FieldValue::Bool(multiple));
        }

        let note = crate::render::render_body(
            &t.body,
            &result.partials,
            &t.fields,
            &state,
            &t.groups,
            &groups_state,
        )
        .expect("the OMS note should render");

        // One diagnosis line per tooth selected, and none for teeth that weren't.
        assert!(note.contains("#1 - symptomatic, PBI"));
        assert!(note.contains("#17 - symptomatic, PBI"));
        assert!(!note.contains("#32 -"));

        // Whole numbers read as counts, not floats.
        assert!(note.contains("total anesthesia time = 45 minutes"));
        assert!(note.contains("epinephrine x 2 carpules"));

        // "a, b and c" joining.
        assert!(note.contains("midazolam, ketamine and propofol"));

        // A speed button filled its own block with prose in declared order,
        // and the plural/singular swap tracked each block's `multiple_teeth`.
        assert!(note.contains("Luxated and extracted each tooth in its entirety"));
        assert!(note.contains("No tooth structure retained in sockets"));
        assert!(note.contains("buccal trough was created adjacent to the tooth"));
        assert!(note.contains("No tooth structure retained in socket."));
        assert!(
            !note.contains("[[each]]") && !note.contains("[[s]]"),
            "placeholders must all be substituted"
        );

        // Sections gated on IVGA appear; the non-IVGA vitals line does not.
        assert!(note.contains("See anesthesia record for details"));
        assert!(!note.contains("BP: WNL"));
        assert!(note.contains("Denies pregnancy."));

        // Airway items read as prose, not as nested lists.
        assert!(note.contains("throat screen and bite block for stabilization were used"));
    }

    /// Regression guard for the bundled `oral_surgery.toml` example, which
    /// demonstrates repeatable groups + speed buttons that add to them:
    /// clicking its multi-tooth speed button should append 8 procedure
    /// instances that then show up in the rendered note.
    #[test]
    fn oral_surgery_speed_button_populates_the_repeatable_group() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/templates");
        let result = load_templates(&dir);
        let template = result
            .templates
            .iter()
            .find(|t| t.id == "oral_surgery")
            .expect("oral_surgery.toml should be a bundled example");
        assert_eq!(template.groups.len(), 1);
        assert_eq!(template.groups[0].key, "procedures");

        let quadrant_button = template
            .speed_buttons
            .iter()
            .find(|sb| sb.group_values.len() == 8)
            .expect("expected an 8-tooth quadrant speed button");

        let mut state = crate::field::build_form_state(&template.fields);
        let mut groups_state = crate::field::build_groups_state(&template.groups);
        crate::field::apply_speed_button(
            quadrant_button,
            &template.fields,
            &mut state,
            &template.groups,
            &mut groups_state,
        );
        assert_eq!(groups_state.get("procedures").unwrap().len(), 8);

        let rendered = crate::render::render_body(
            &template.body,
            &result.partials,
            &template.fields,
            &state,
            &template.groups,
            &groups_state,
        )
        .unwrap();
        assert!(rendered.contains("Tooth 1"));
        assert!(rendered.contains("Tooth 8"));
        assert!(!rendered.contains("No procedure steps recorded"));
    }

    /// Regression guard: the bundled templates' `**bold**` markers (in
    /// partials/header.tera and the templates themselves) must survive Tera
    /// rendering intact and actually parse as bold — this is what would
    /// catch e.g. a template accidentally escaping or mangling the markers.
    #[test]
    fn bundled_templates_render_actual_bold_formatting() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/templates");
        let result = load_templates(&dir);
        let template = result
            .templates
            .iter()
            .find(|t| t.id == "follow_up_visit")
            .unwrap();

        let state = crate::field::build_form_state(&template.fields);
        let groups_state = crate::field::build_groups_state(&template.groups);
        let rendered = crate::render::render_body(
            &template.body,
            &result.partials,
            &template.fields,
            &state,
            &template.groups,
            &groups_state,
        )
        .unwrap();

        let runs = crate::formatting::parse(&rendered);
        assert!(
            runs.iter().any(|r| r.bold && r.text.contains("Patient:")),
            "expected the included header partial's \"Patient:\" label to render bold"
        );
    }
}
