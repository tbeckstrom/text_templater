//! Static checks over loaded templates — the mistakes that load and render
//! perfectly well but mean the form isn't doing what the author intended: a
//! field nothing reads, a speed button setting a key that no longer exists, a
//! condition referring to a renamed field.
//!
//! Everything here is a *warning*. A template that trips a lint still works;
//! the point is to surface drift after an edit.

use std::collections::HashSet;

use crate::template::{FieldDef, FieldType, GroupDef, PartialDef, TemplateDef};

#[derive(Debug, Clone, PartialEq)]
pub struct Lint {
    /// Template id, or `partials/<name>` for a partial-level finding.
    pub subject: String,
    pub message: String,
}

/// Words that appear in Tera expressions but aren't form fields — keywords,
/// the builtin filters and functions this app's templates actually use, and
/// the keyword-argument names those filters take.
const TERA_WORDS: &[&str] = &[
    "if", "else", "elif", "endif", "for", "endfor", "in", "set", "set_global", "and", "or", "not",
    "is", "as", "true", "false", "loop", "index", "index0", "first", "last", "length", "include",
    "block", "endblock", "macro", "endmacro", "component", "endcomponent", "filter", "endfilter",
    "raw", "endraw", "join", "split", "trim", "trim_start", "trim_end", "replace", "int", "float",
    "str", "abs", "round", "reverse", "sort", "unique", "get", "values", "keys", "pairs",
    "group_by", "upper", "lower", "title", "capitalize", "truncate", "indent", "default", "safe",
    "wordcount", "nth", "range", "throw", "pluralize", "escape_html", "escape_xml",
    "newlines_to_br", "sep", "pat", "from", "to", "with", "start", "end", "count", "step_by",
    "attribute", "case_sensitive", "n", "d", "s", "defined", "undefined",
];

/// Pulls the identifier-looking words out of a Tera expression, ignoring the
/// contents of string literals (so `'Lidocaine 2% w/ epi' in local_used`
/// reports only `local_used`), the loop variable of a list comprehension
/// (so `[t for t in teeth if t in ["36"]]` reports only `teeth`), and any name
/// the expression tests with `is defined` (`x is defined and x == "Yes"`),
/// which says the field is expected to be missing from some notes.
fn identifiers(expr: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut chars = expr.chars().peekable();

    while let Some(c) = chars.next() {
        match quote {
            Some(q) => {
                if c == '\\' {
                    chars.next();
                } else if c == q {
                    quote = None;
                }
            }
            None if c == '\'' || c == '"' => quote = Some(c),
            None if c.is_alphanumeric() || c == '_' => current.push(c),
            None => {
                push_word(&mut words, &mut current);
                // A dotted access (`b.tooth`) only names the head; skip the
                // member so it isn't mistaken for a field of its own.
                if c == '.' {
                    while chars.peek().is_some_and(|n| n.is_alphanumeric() || *n == '_') {
                        chars.next();
                    }
                }
            }
        }
    }
    push_word(&mut words, &mut current);

    let bound: Vec<String> = words
        .windows(2)
        .filter(|pair| pair[0] == "for")
        .map(|pair| pair[1].clone())
        .collect();
    let guarded: Vec<String> = words
        .windows(4)
        .chain(words.windows(3))
        .filter(|w| {
            w[1] == "is"
                && (matches!(w[2].as_str(), "defined" | "undefined")
                    || (w[2] == "not" && w.get(3).is_some_and(|d| d == "defined")))
        })
        .map(|w| w[0].clone())
        .collect();
    words
        .into_iter()
        .filter(|w| !TERA_WORDS.contains(&w.as_str()) && !bound.contains(w) && !guarded.contains(w))
        .collect()
}

fn push_word(out: &mut Vec<String>, current: &mut String) {
    let word = std::mem::take(current);
    if word.is_empty() || word.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return;
    }
    out.push(word);
}

/// Whether `text` mentions `key` as a whole word — used to decide if anything
/// actually reads a field.
fn mentions(text: &str, key: &str) -> bool {
    let bytes = text.as_bytes();
    let mut from = 0;
    while let Some(pos) = text[from..].find(key) {
        let start = from + pos;
        let end = start + key.len();
        let before_ok = start == 0 || !is_word_byte(bytes[start - 1]);
        let after_ok = end == bytes.len() || !is_word_byte(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        from = start + key.len();
    }
    false
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Whether nothing reads `field`.
///
/// A field counts as read via its own key or via the `<key>_text` expansion
/// name, which is what a template using option prose actually references. A
/// `computed` field is exempt: showing a derived value in the form is a
/// complete purpose on its own, so never appearing in the body is normal.
fn is_unread(readable: &str, field: &FieldDef) -> bool {
    if field.field_type == FieldType::Computed {
        return false;
    }
    !mentions(readable, &field.key)
        && !mentions(readable, &format!("{}_text", field.key))
        && !mentions(readable, &format!("{}_spans", field.key))
}

/// Names a template's expressions may legitimately mention: its fields, the
/// `<key>_text` expansions option fields publish, its groups, and (inside a
/// group) that group's own fields and `source_as` name.
fn known_names(fields: &[FieldDef], groups: &[GroupDef]) -> HashSet<String> {
    let mut names = HashSet::new();
    for f in fields {
        names.insert(f.key.clone());
        if f.has_option_text() {
            names.insert(format!("{}_text", f.key));
        }
        if f.field_type == FieldType::Teeth {
            names.insert(format!("{}_spans", f.key));
        }
    }
    for g in groups {
        names.insert(g.key.clone());
    }
    names
}

/// Every partial `body` includes, directly or through another partial.
fn included_partials<'a>(body: &str, partials: &'a [PartialDef]) -> Vec<&'a PartialDef> {
    let mut found: Vec<&PartialDef> = Vec::new();
    let mut to_scan = vec![body];
    while let Some(text) = to_scan.pop() {
        for p in partials {
            if !found.iter().any(|f| f.name == p.name) && text.contains(&format!("\"{}\"", p.name)) {
                found.push(p);
                to_scan.push(&p.body);
            }
        }
    }
    found
}

/// Checks the loaded templates and partials, returning every warning found.
pub fn lint(templates: &[TemplateDef], partials: &[PartialDef]) -> Vec<Lint> {
    let mut lints = Vec::new();

    for t in templates {
        // Everything this template's text could read from: its own body plus
        // the body of any partial it includes.
        let mut readable = t.body.clone();
        for p in included_partials(&t.body, partials) {
            readable.push('\n');
            readable.push_str(&p.body);
        }
        // Conditions and computed expressions count as reads too.
        for f in t.fields.iter().chain(t.groups.iter().flat_map(|g| &g.fields)) {
            for expr in [f.visible_if.as_deref(), f.compute.as_deref()].into_iter().flatten() {
                readable.push('\n');
                readable.push_str(expr);
            }
        }
        for g in &t.groups {
            if let Some(expr) = &g.visible_if {
                readable.push('\n');
                readable.push_str(expr);
            }
        }
        for sb in t.speed_buttons.iter().chain(t.groups.iter().flat_map(|g| &g.speed_buttons)) {
            if let Some(expr) = &sb.visible_if {
                readable.push('\n');
                readable.push_str(expr);
            }
        }
        // Driving a group is a use of the multiselect behind it.
        for g in &t.groups {
            if let Some(source) = &g.source {
                readable.push('\n');
                readable.push_str(source);
            }
        }

        let top_level = known_names(&t.fields, &t.groups);

        for f in &t.fields {
            if is_unread(&readable, f) {
                lints.push(Lint {
                    subject: t.id.clone(),
                    message: format!(
                        "field '{}' is never read by the body or any expression",
                        f.key
                    ),
                });
            }
            check_expressions(&mut lints, &t.id, f, &top_level);
        }

        for g in &t.groups {
            if !mentions(&readable, &g.key) {
                lints.push(Lint {
                    subject: t.id.clone(),
                    message: format!("group '{}' is never looped over in the body", g.key),
                });
            }
            // Inside a block, its own fields and the source label are in scope.
            let mut scope = top_level.clone();
            scope.extend(known_names(&g.fields, &[]));
            scope.insert(g.source_as_key().to_string());
            for f in &g.fields {
                check_expressions(&mut lints, &t.id, f, &scope);
            }

            if let Some(source) = &g.source {
                match t.fields.iter().find(|f| &f.key == source) {
                    None => lints.push(Lint {
                        subject: t.id.clone(),
                        message: format!(
                            "group '{}' is driven by '{source}', which is not a field",
                            g.key
                        ),
                    }),
                    Some(f)
                        if !matches!(
                            f.field_type,
                            FieldType::Multiselect | FieldType::Teeth | FieldType::Dropdown
                        ) =>
                    {
                        lints.push(Lint {
                            subject: t.id.clone(),
                            message: format!(
                                "group '{}' is driven by '{source}', which is a {:?} — only \
                                 multiselect/dropdown can drive a group",
                                g.key, f.field_type
                            ),
                        })
                    }
                    Some(_) => {}
                }
            }
        }

        // A chart can only share its odontogram with another tooth list.
        for f in &t.fields {
            let Some(linked) = &f.linked else { continue };
            let ok = f.field_type == FieldType::Teeth
                && t.fields.iter().any(|o| &o.key == linked && o.field_type == FieldType::Teeth);
            if !ok {
                lints.push(Lint {
                    subject: t.id.clone(),
                    message: format!(
                        "field '{}' links to '{linked}', but both must be `teeth` fields",
                        f.key
                    ),
                });
            }
        }

        // A section name that matches no field's section is almost always a
        // typo: the button or group quietly falls back to the top/end.
        let has_section = |name: &str| t.fields.iter().any(|f| f.section.as_deref() == Some(name));
        for (what, label, section) in t
            .speed_buttons
            .iter()
            .map(|sb| ("speed button", &sb.label, &sb.section))
            .chain(t.groups.iter().map(|g| ("group", &g.key, &g.section)))
        {
            if let Some(name) = section
                && !has_section(name)
            {
                lints.push(Lint {
                    subject: t.id.clone(),
                    message: format!(
                        "{what} '{label}' names section '{name}', which no field uses"
                    ),
                });
            }
        }

        // Same for the sections the printed sheet leaves out: a typo'd name
        // quietly prints the section after all.
        for name in &t.print_skip {
            if !has_section(name) {
                lints.push(Lint {
                    subject: t.id.clone(),
                    message: format!("print_skip names section '{name}', which no field uses"),
                });
            }
        }
        for f in t.fields.iter().chain(t.groups.iter().flat_map(|g| &g.fields)) {
            if f.print_lines.is_some()
                && !matches!(f.field_type, FieldType::Text | FieldType::Textarea)
            {
                lints.push(Lint {
                    subject: t.id.clone(),
                    message: format!(
                        "field '{}' sets print_lines, which only text and textarea fields use",
                        f.key
                    ),
                });
            }
        }

        // A speed button that sets a key no field has does nothing at all.
        for sb in &t.speed_buttons {
            for key in sb.values.keys() {
                if !t.fields.iter().any(|f| &f.key == key) {
                    lints.push(Lint {
                        subject: t.id.clone(),
                        message: format!(
                            "speed button '{}' sets '{key}', which is not a field",
                            sb.label
                        ),
                    });
                }
            }
            for (group_key, values) in &sb.each {
                let Some(g) = t.groups.iter().find(|g| &g.key == group_key) else {
                    lints.push(Lint {
                        subject: t.id.clone(),
                        message: format!(
                            "speed button '{}' sets values in '{group_key}', which is not a group",
                            sb.label
                        ),
                    });
                    continue;
                };
                for key in values.as_table().into_iter().flat_map(|v| v.keys()) {
                    if !g.fields.iter().any(|f| &f.key == key) {
                        lints.push(Lint {
                            subject: t.id.clone(),
                            message: format!(
                                "speed button '{}' sets '{key}', which is not a field of group \
                                 '{group_key}'",
                                sb.label
                            ),
                        });
                    }
                }
            }
        }
        for g in &t.groups {
            for sb in &g.speed_buttons {
                for key in sb.values.keys() {
                    if !g.fields.iter().any(|f| &f.key == key) {
                        lints.push(Lint {
                            subject: t.id.clone(),
                            message: format!(
                                "speed button '{}' in group '{}' sets '{key}', which is not a \
                                 field of that group",
                                sb.label, g.key
                            ),
                        });
                    }
                }
            }
        }
    }

    // A partial nobody includes is usually a leftover or a typo'd name.
    for p in partials {
        let used = templates
            .iter()
            .any(|t| included_partials(&t.body, partials).iter().any(|i| i.name == p.name));
        if !used {
            lints.push(Lint {
                subject: format!("partials/{}", p.name),
                message: "no template includes this partial".to_string(),
            });
        }
    }

    lints
}

fn check_expressions(lints: &mut Vec<Lint>, subject: &str, field: &FieldDef, known: &HashSet<String>) {
    for (what, expr) in [
        ("visible_if", field.visible_if.as_deref()),
        ("compute", field.compute.as_deref()),
    ] {
        let Some(expr) = expr else { continue };
        for name in identifiers(expr) {
            if !known.contains(&name) {
                lints.push(Lint {
                    subject: subject.to_string(),
                    message: format!(
                        "{what} on '{}' refers to '{name}', which is not a field",
                        field.key
                    ),
                });
            }
        }
    }
    if field.field_type == FieldType::Computed && field.compute.is_none() {
        lints.push(Lint {
            subject: subject.to_string(),
            message: format!("computed field '{}' has no `compute` expression", field.key),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(key: &str, ty: FieldType) -> FieldDef {
        FieldDef {
            key: key.to_string(),
            label: key.to_string(),
            field_type: ty,
            ..Default::default()
        }
    }

    fn template(body: &str, fields: Vec<FieldDef>) -> TemplateDef {
        TemplateDef {
            id: "t".to_string(),
            name: "T".to_string(),
            description: None,
            fields,
            groups: vec![],
            speed_buttons: vec![],
            print_skip: vec![],
            body: body.to_string(),
        }
    }

    #[test]
    fn identifiers_ignore_string_literals_and_dotted_members() {
        let names = identifiers("'Lidocaine 2% w/ epi' in local_used and b.tooth");
        assert_eq!(names, vec!["local_used".to_string(), "b".to_string()]);
    }

    #[test]
    fn identifiers_skip_a_list_comprehension_loop_variable() {
        let names = identifiers(r#""1" if [t for t in sites if t in ["36", "46"]] | length > 0 else """#);
        assert_eq!(names, vec!["sites".to_string()]);
    }

    #[test]
    fn identifiers_skip_a_name_guarded_by_is_defined() {
        let names = identifiers(
            r#""x" if (hx_tobacco is defined and hx_tobacco == "Current") or ap is not defined or "T" in manual else """#,
        );
        assert_eq!(names, vec!["manual".to_string()]);
    }

    #[test]
    fn identifiers_skip_tera_keywords_and_filters() {
        let names = identifiers("items | join(sep=\", \") | length");
        assert_eq!(names, vec!["items".to_string()]);
    }

    #[test]
    fn a_field_read_by_the_body_is_not_flagged() {
        let t = template("{{ name }}", vec![field("name", FieldType::Text)]);
        assert!(lint(&[t], &[]).is_empty());
    }

    #[test]
    fn a_field_nothing_reads_is_flagged() {
        let t = template("static text", vec![field("orphan", FieldType::Text)]);
        let lints = lint(&[t], &[]);
        assert_eq!(lints.len(), 1);
        assert!(lints[0].message.contains("orphan"));
    }

    #[test]
    fn a_field_read_only_by_a_condition_counts_as_used() {
        let mut gate = field("gate", FieldType::Checkbox);
        let mut shown = field("shown", FieldType::Text);
        shown.visible_if = Some("gate".to_string());
        gate.visible_if = None;
        let t = template("{{ shown }}", vec![gate, shown]);
        let lints = lint(std::slice::from_ref(&t), &[]);
        assert!(lints.is_empty(), "unexpected lints: {lints:?}");
    }

    #[test]
    fn a_condition_naming_an_unknown_field_is_flagged() {
        let mut f = field("shown", FieldType::Text);
        f.visible_if = Some("typoed_key".to_string());
        let t = template("{{ shown }}", vec![f]);
        let lints = lint(&[t], &[]);
        assert_eq!(lints.len(), 1);
        assert!(lints[0].message.contains("typoed_key"));
    }

    #[test]
    fn a_speed_button_setting_an_unknown_key_is_flagged() {
        let mut t = template("{{ real }}", vec![field("real", FieldType::Text)]);
        t.speed_buttons = vec![crate::template::SpeedButtonDef {
            label: "Preset".to_string(),
            values: [("gone".to_string(), toml::Value::String("x".into()))]
                .into_iter()
                .collect(),
            ..Default::default()
        }];
        let lints = lint(&[t], &[]);
        assert_eq!(lints.len(), 1);
        assert!(lints[0].message.contains("gone"));
    }

    #[test]
    fn a_speed_button_setting_an_unknown_group_or_group_field_is_flagged() {
        let mut t = template("{% for b in blocks %}{{ b.real }}{% endfor %}", vec![]);
        t.groups = vec![crate::template::GroupDef {
            key: "blocks".to_string(),
            label: "Block".to_string(),
            fields: vec![field("real", FieldType::Text)],
            ..Default::default()
        }];
        let each: toml::Table = toml::from_str(
            "blocks = { real = \"x\", gone = \"x\" }\nmissing = { real = \"x\" }",
        )
        .unwrap();
        t.speed_buttons = vec![crate::template::SpeedButtonDef {
            label: "Preset".to_string(),
            each,
            ..Default::default()
        }];
        let lints = lint(&[t], &[]);
        assert_eq!(lints.len(), 2, "{lints:?}");
        assert!(lints.iter().any(|l| l.message.contains("'gone'")));
        assert!(lints.iter().any(|l| l.message.contains("'missing'")));
    }

    #[test]
    fn a_speed_button_naming_an_unknown_section_is_flagged() {
        let mut f = field("real", FieldType::Text);
        f.section = Some("History".to_string());
        let mut t = template("{{ real }}", vec![f]);
        t.speed_buttons = vec![
            crate::template::SpeedButtonDef {
                label: "Here".to_string(),
                section: Some("History".to_string()),
                ..Default::default()
            },
            crate::template::SpeedButtonDef {
                label: "Lost".to_string(),
                section: Some("Histroy".to_string()),
                ..Default::default()
            },
        ];
        let lints = lint(&[t], &[]);
        assert_eq!(lints.len(), 1, "got {lints:?}");
        assert!(lints[0].message.contains("Histroy"));
    }

    #[test]
    fn a_group_driven_by_a_non_multiselect_is_flagged() {
        let mut t = template("{{ blocks }}", vec![field("note", FieldType::Text)]);
        t.groups = vec![GroupDef {
            key: "blocks".to_string(),
            label: "Block".to_string(),
            source: Some("note".to_string()),
            ..Default::default()
        }];
        let lints = lint(&[t], &[]);
        assert!(
            lints.iter().any(|l| l.message.contains("only multiselect")),
            "got {lints:?}"
        );
    }

    #[test]
    fn a_computed_field_without_an_expression_is_flagged() {
        let t = template("{{ total }}", vec![field("total", FieldType::Computed)]);
        let lints = lint(&[t], &[]);
        assert!(lints.iter().any(|l| l.message.contains("no `compute`")));
    }

    #[test]
    fn a_field_read_through_its_text_expansion_counts_as_used() {
        let mut f = field("socket", FieldType::Multiselect);
        f.options = vec![crate::template::OptionDef {
            label: "Irrigated".to_string(),
            text: Some("Irrigated the site".to_string()),
        }];
        // The body references the expansion, never the bare key.
        let t = template("{{ socket_text | join(sep=\". \") }}", vec![f]);
        let lints = lint(std::slice::from_ref(&t), &[]);
        assert!(lints.is_empty(), "unexpected lints: {lints:?}");
    }

    #[test]
    fn a_multiselect_that_only_drives_a_group_counts_as_used() {
        let mut t = template("{% for d in diagnoses %}{{ d.item }}{% endfor %}", vec![]);
        t.fields = vec![field("teeth", FieldType::Multiselect)];
        t.groups = vec![GroupDef {
            key: "diagnoses".to_string(),
            label: "Diagnosis".to_string(),
            source: Some("teeth".to_string()),
            ..Default::default()
        }];
        let lints = lint(std::slice::from_ref(&t), &[]);
        assert!(lints.is_empty(), "unexpected lints: {lints:?}");
    }

    #[test]
    fn a_display_only_computed_field_is_not_flagged_as_unread() {
        let mut f = field("total", FieldType::Computed);
        f.compute = Some("a + b".to_string());
        let mut t = template("no mention of it", vec![f]);
        t.fields.push(field("a", FieldType::Number));
        t.fields.push(field("b", FieldType::Number));
        let lints = lint(std::slice::from_ref(&t), &[]);
        // `a` and `b` are genuinely unread by the body, but they *are* read by
        // the compute expression, so nothing should be flagged.
        assert!(lints.is_empty(), "unexpected lints: {lints:?}");
    }

    #[test]
    fn an_unused_partial_is_flagged_but_a_used_one_is_not() {
        let used = PartialDef {
            name: "header".to_string(),
            body: "hi".to_string(),
            ..Default::default()
        };
        let unused = PartialDef {
            name: "leftover".to_string(),
            body: "x".to_string(),
            ..Default::default()
        };
        let t = template(r#"{% include "header" %}"#, vec![]);
        let lints = lint(&[t], &[used, unused]);
        assert_eq!(lints.len(), 1);
        assert_eq!(lints[0].subject, "partials/leftover");
    }

    #[test]
    fn a_field_read_only_inside_an_included_partial_counts_as_used() {
        let partial = PartialDef {
            name: "header".to_string(),
            body: "{{ patient_name }}".to_string(),
            ..Default::default()
        };
        let t = template(
            r#"{% include "header" %}"#,
            vec![field("patient_name", FieldType::Text)],
        );
        assert!(lint(&[t], &[partial]).is_empty());
    }

    #[test]
    fn a_partial_included_only_by_another_partial_is_used_and_its_reads_count() {
        let outer = PartialDef {
            name: "approach".to_string(),
            body: r#"{% include "local" %}"#.to_string(),
            ..Default::default()
        };
        let inner = PartialDef {
            name: "local".to_string(),
            body: "{{ local_agent }}".to_string(),
            ..Default::default()
        };
        let t = template(
            r#"{% include "approach" %}"#,
            vec![field("local_agent", FieldType::Text)],
        );
        let lints = lint(&[t], &[outer, inner]);
        assert!(lints.is_empty(), "unexpected lints: {lints:?}");
    }

    #[test]
    fn print_skip_naming_no_section_and_misplaced_print_lines_are_flagged() {
        let mut sectioned = field("hygiene", FieldType::Text);
        sectioned.section = Some("Exam".to_string());
        let mut picky = field("occlusion", FieldType::Dropdown);
        picky.print_lines = Some(2);
        let mut t = template("{{ hygiene }} {{ occlusion }}", vec![sectioned, picky]);
        t.print_skip = vec!["Exam".to_string(), "Imaging".to_string()];
        let messages: Vec<_> = lint(&[t], &[]).into_iter().map(|l| l.message).collect();
        assert_eq!(
            messages,
            vec![
                "print_skip names section 'Imaging', which no field uses",
                "field 'occlusion' sets print_lines, which only text and textarea fields use",
            ]
        );
    }
}
