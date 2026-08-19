use std::fs;
use std::path::Path;

use super::{FieldDef, RawTemplateFile, SharedFieldsFile, TemplateDef};

pub struct LoadResult {
    pub templates: Vec<TemplateDef>,
    /// (name, Tera source) for each reusable body section in `partials/`,
    /// includable from any template's `body` via `{% include "name" %}`.
    pub partials: Vec<(String, String)>,
    /// (file name, error message) for any template or partial that failed to load.
    pub errors: Vec<(String, String)>,
}

pub fn load_templates(dir: &Path) -> LoadResult {
    let mut errors = Vec::new();

    let shared_fields = load_shared_fields(dir, &mut errors);
    let partials = load_partials(dir, &mut errors);

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

        match load_one(&path, &shared_fields) {
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

/// Loads every `*.tera` file in the reserved `partials/` subdirectory as a
/// named, includable body section (name = file stem). Missing directory is
/// fine (no partials); a file that can't be read is reported as an error but
/// doesn't stop the rest from loading.
fn load_partials(dir: &Path, errors: &mut Vec<(String, String)>) -> Vec<(String, String)> {
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

    let mut partials = Vec::new();
    for path in paths {
        if path.extension().and_then(|s| s.to_str()) != Some("tera") {
            continue;
        }
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        match fs::read_to_string(&path) {
            Ok(content) => partials.push((name, content)),
            Err(e) => errors.push((format!("partials/{name}.tera"), e.to_string())),
        }
    }
    partials
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

fn load_one(path: &Path, shared_fields: &[FieldDef]) -> Result<TemplateDef, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let raw: RawTemplateFile = toml::from_str(&text).map_err(|e| e.to_string())?;

    let mut fields = Vec::with_capacity(raw.use_shared.len() + raw.fields.len());
    for key in &raw.use_shared {
        let field = shared_fields
            .iter()
            .find(|f| &f.key == key)
            .ok_or_else(|| {
                format!("use_shared references unknown key '{key}' (not found in shared_fields.toml)")
            })?;
        fields.push(field.clone());
    }
    fields.extend(raw.fields);

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
        groups: raw.groups,
        speed_buttons: raw.speed_buttons,
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
        assert_eq!(result.templates.len(), 3);
        assert!(
            result.partials.iter().any(|(name, _)| name == "header"),
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
            assert!(
                rendered.unwrap().contains("Patient:"),
                "expected the included header partial's content in '{}'",
                template.id
            );
        }
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
