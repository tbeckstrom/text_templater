//! The printed sheet: a paper copy of a template's form, filled in by hand in
//! the room and entered into the app afterwards. It mirrors the form (same
//! sections, labels and order), so entering it is a matter of reading down
//! the sheet while clicking down the form.
//!
//! What it shows:
//! - A default choice prints **bold and unmarked**, meaning "assumed unless
//!   marked". A choice already made in the app that differs from the default
//!   prints filled. A checkbox has only one mark, so it prints as it stands.
//! - Conditional (`visible_if`) fields always print, indented under the field
//!   before them, since the answer they hang on can change in the room.
//! - A computed value prints only when marked `print = true`, as tick-boxes
//!   (one per item of a list): talking points such as the risks to cover.
//!   Nothing marked `print = false` or sitting in a section named in the
//!   note's `print_skip` prints.
//! - A selection-driven group prints one box per instance. With none yet
//!   (sites not charted), it prints blank boxes to write the site in.
//!
//! The output is a standalone HTML document with its own print styles. It
//! uses no browser APIs, so any build can hand it to something that prints.

use std::collections::HashSet;
use std::fmt::Write as _;

use crate::field::{build_form_state, FieldValue, FormState, GroupsState};
use crate::template::{is_primary_tooth, FieldDef, FieldType, GroupDef, SpeedButtonDef, TemplateDef, ARCHES};

/// Write-in lines under "Notes / questions" at the end of the sheet.
const NOTES_LINES: usize = 6;

/// Builds the sheet for `template` as a complete HTML document, pre-filled
/// from the current form values.
pub fn sheet_html(
    template: &TemplateDef,
    state: &FormState,
    groups_state: &GroupsState,
    printed_on: jiff::civil::Date,
) -> String {
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{}</title><style>{STYLE}</style></head>\
         <body>{}</body></html>",
        esc(&template.name),
        sheet_body(template, state, groups_state, printed_on),
    )
}

/// The sheet's content without the document around it, for a page that
/// brings its own and applies [`STYLE`] (the web build adds it to the app's
/// page to print).
pub fn sheet_body(
    template: &TemplateDef,
    state: &FormState,
    groups_state: &GroupsState,
    printed_on: jiff::civil::Date,
) -> String {
    let skipped = |section: Option<&str>| section.is_some_and(|s| template.print_skip.iter().any(|k| k == s));
    let in_field_section = |section: Option<&str>| {
        section.is_some_and(|name| template.fields.iter().any(|f| f.section.as_deref() == Some(name)))
    };

    let mut out = String::new();
    let _ = write!(
        out,
        "<header><h1>{name}</h1><span>Printed {printed_on}</span></header>\
         <p class=\"legend\"><span class=\"def\">Bold underlined</span> = default, assumed unless marked. \
         Filled = already set in the app.</p>",
        name = esc(&template.name),
    );

    // The form's top row of speed buttons: the pathway presets.
    let top_row: Vec<&SpeedButtonDef> = template
        .speed_buttons
        .iter()
        .filter(|sb| !in_field_section(sb.section.as_deref()) && !skipped(sb.section.as_deref()))
        .collect();
    if !top_row.is_empty() {
        out.push_str("<div class=\"presets\">");
        buttons_html(&mut out, &top_row);
        out.push_str("</div>");
    }

    out.push_str("<main>");
    // Same walk as the form: each run of consecutive fields sharing a section
    // under one heading, with that section's buttons above its fields and its
    // groups after them.
    let mut printed_groups: HashSet<&str> = HashSet::new();
    let mut i = 0;
    while i < template.fields.len() {
        let section = template.fields[i].section.as_deref();
        let mut end = i;
        while end < template.fields.len() && template.fields[end].section.as_deref() == section {
            end += 1;
        }
        let run = &template.fields[i..end];
        i = end;

        let Some(name) = section else {
            for field in run {
                field_html(&mut out, field, &template.fields, state);
            }
            continue;
        };
        if skipped(Some(name)) {
            continue;
        }

        let mut body = String::new();
        let here: Vec<&SpeedButtonDef> = template
            .speed_buttons
            .iter()
            .filter(|sb| sb.section.as_deref() == Some(name))
            .collect();
        if !here.is_empty() {
            body.push_str("<div class=\"btns\">");
            buttons_html(&mut body, &here);
            body.push_str("</div>");
        }
        for field in run {
            field_html(&mut body, field, &template.fields, state);
        }
        // A section split into two runs gets its groups once, in the first.
        for group in template.groups.iter().filter(|g| g.section.as_deref() == Some(name)) {
            if printed_groups.insert(&group.key) {
                group_html(&mut body, group, groups_state);
            }
        }
        if !body.is_empty() {
            let _ = write!(out, "<h2>{}</h2>{body}", esc(name));
        }
    }
    for group in &template.groups {
        let section = group.section.as_deref();
        if !in_field_section(section) && !skipped(section) {
            group_html(&mut out, group, groups_state);
        }
    }

    out.push_str("<section class=\"notes\"><h2>Notes / questions</h2>");
    for _ in 0..NOTES_LINES {
        out.push_str("<div class=\"line\"></div>");
    }
    out.push_str("</section></main>");
    out
}

fn buttons_html(out: &mut String, buttons: &[&SpeedButtonDef]) {
    for sb in buttons {
        let _ = write!(out, "<span class=\"o\">&#9744; {}</span> ", esc(&sb.label));
    }
}

/// One field, pre-filled from `state`. `siblings` is the list it came from,
/// which a tooth chart looks through for the list it's linked with.
fn field_html(out: &mut String, field: &FieldDef, siblings: &[FieldDef], state: &FormState) {
    if field.field_type == FieldType::Computed {
        if field.print == Some(true) {
            checklist_html(out, field, state);
        }
        return;
    }
    if !field.prints() {
        return;
    }
    // A tooth list linked from another chart is drawn as part of that chart.
    if siblings.iter().any(|f| f.linked.as_deref() == Some(field.key.as_str())) {
        return;
    }

    let default = FieldValue::default_for(field);
    let value = state.get(&field.key).cloned().unwrap_or_else(|| default.clone());
    let is_default = value.to_json() == default.to_json();
    let class = if field.visible_if.is_some() { "f cond" } else { "f" };
    let label = esc(field.sheet_label());

    match field.field_type {
        FieldType::Computed => {}
        FieldType::Checkbox => {
            // Only "assumed ticked" is worth emphasizing; every box is
            // assumed unticked unless marked.
            let on = matches!(value, FieldValue::Bool(true));
            let _ = write!(
                out,
                "<div class=\"{class}\"><span class=\"o{}\">{} {label}</span></div>",
                if is_default && on { " def" } else { "" },
                if on { "&#9745;" } else { "&#9744;" },
            );
        }
        FieldType::Dropdown | FieldType::Multiselect => {
            let (empty, filled) = if field.field_type == FieldType::Dropdown {
                ("&#9675;", "&#9679;")
            } else {
                ("&#9744;", "&#9632;")
            };
            let picked = picks(&value);
            let defaults = picks(&default);
            let _ = write!(out, "<div class=\"{class}\"><span class=\"l\">{label}</span> ");
            for option in field.option_labels() {
                let on = !is_default && picked.contains(&option);
                let def = if defaults.contains(&option) { " def" } else { "" };
                let _ = write!(
                    out,
                    "<span class=\"o{def}\">{} {}</span> ",
                    if on { filled } else { empty },
                    esc(option),
                );
            }
            out.push_str("</div>");
        }
        FieldType::Teeth => {
            let linked = field
                .linked
                .as_ref()
                .and_then(|key| siblings.iter().find(|f| &f.key == key));
            chart_html(out, field, linked, state);
        }
        FieldType::Text | FieldType::Textarea | FieldType::Number | FieldType::Date => {
            let written = written_value(field, &value);
            let def = if is_default && !written.is_empty() { " def" } else { "" };
            let lines = field.print_lines.unwrap_or(if field.field_type == FieldType::Textarea { 3 } else { 1 });
            if lines <= 1 {
                let _ = write!(
                    out,
                    "<div class=\"{class}\"><span class=\"l\">{label}</span> <span class=\"blank{def}\">{}</span></div>",
                    esc(&written),
                );
            } else {
                let _ = write!(
                    out,
                    "<div class=\"{class}\"><span class=\"l\">{label}</span><div class=\"line{def}\">{}</div>",
                    esc(&written),
                );
                for _ in 1..lines {
                    out.push_str("<div class=\"line\"></div>");
                }
                out.push_str("</div>");
            }
        }
    }
}

/// A computed value as tick-boxes under its label, one per list item, to
/// tick off as each is covered. With nothing to list yet, one blank box.
fn checklist_html(out: &mut String, field: &FieldDef, state: &FormState) {
    let items = match state.get(&field.key) {
        Some(FieldValue::Text(s)) if !s.trim().is_empty() => vec![s.trim()],
        Some(FieldValue::MultiSelect(v)) => v.iter().map(|s| s.as_str()).collect(),
        _ => Vec::new(),
    };
    let _ = write!(out, "<div class=\"f\"><div class=\"l\">{}</div>", esc(field.sheet_label()));
    if items.is_empty() {
        out.push_str("<div class=\"tick\">&#9744; <span class=\"blank\"></span></div>");
    }
    for item in items {
        let _ = write!(out, "<div class=\"tick\">&#9744; {}</div>", esc(item));
    }
    out.push_str("</div>");
}

/// The picked option labels in a choice field's value.
fn picks(value: &FieldValue) -> Vec<&str> {
    match value {
        FieldValue::Text(s) if !s.is_empty() => vec![s.as_str()],
        FieldValue::MultiSelect(v) => v.iter().map(String::as_str).collect(),
        _ => Vec::new(),
    }
}

/// What a write-in field already holds, as it goes on the line. `***` (the
/// "fill in later" flag) prints as an empty line, and so does a number that
/// was never given a value.
fn written_value(field: &FieldDef, value: &FieldValue) -> String {
    let text = match value {
        FieldValue::Text(s) => s.trim().to_string(),
        FieldValue::Date(d) => d.text.trim().to_string(),
        FieldValue::Number(n) if *n == 0.0 && field.default.is_none() => String::new(),
        FieldValue::Number(n) if n.fract() == 0.0 => format!("{n:.0}"),
        FieldValue::Number(n) => n.to_string(),
        FieldValue::Bool(_) | FieldValue::MultiSelect(_) => String::new(),
    };
    if text == "***" { String::new() } else { text }
}

/// An FDI chart to mark teeth on: a circle for `field`'s teeth and, when it
/// shares the chart with `linked`, a box for that list's. Teeth already
/// charted in the app print marked. Primary rows only print once a primary
/// tooth is charted, as in the form.
fn chart_html(out: &mut String, field: &FieldDef, linked: Option<&FieldDef>, state: &FormState) {
    let list = |key: &str| match state.get(key) {
        Some(FieldValue::MultiSelect(v)) => v.clone(),
        _ => Vec::new(),
    };
    let own = list(&field.key);
    let other = linked.map(|f| list(&f.key)).unwrap_or_default();
    let offered = |tooth: &str| field.options.iter().any(|o| o.label == tooth);

    let _ = write!(out, "<div class=\"f chart\"><div class=\"l\">Circle = {}", esc(field.sheet_label()));
    if let Some(l) = linked {
        let _ = write!(out, "; box = {}", esc(l.sheet_label()));
    }
    out.push_str("</div><table>");
    let any_primary = own.iter().chain(&other).any(|t| is_primary_tooth(t));
    for arch in ARCHES.iter().filter(|arch| any_primary || !is_primary_tooth(arch[0])) {
        out.push_str("<tr>");
        for (i, &tooth) in arch.iter().enumerate() {
            if i == arch.len() / 2 {
                out.push_str("<td class=\"mid\"></td>");
            }
            if !offered(tooth) {
                out.push_str("<td></td>");
                continue;
            }
            let class = if own.iter().any(|t| t == tooth) {
                " class=\"own\""
            } else if other.iter().any(|t| t == tooth) {
                " class=\"lnk\""
            } else {
                ""
            };
            let _ = write!(out, "<td{class}>{tooth}</td>");
        }
        out.push_str("</tr>");
    }
    out.push_str("</table></div>");
}

/// A repeatable group as one box per instance. With no instances yet, blank
/// boxes to write the site in: two for a group of spans, one otherwise.
fn group_html(out: &mut String, group: &GroupDef, groups_state: &GroupsState) {
    if !group.prints() {
        return;
    }
    let label = esc(group.sheet_label());
    let instances = groups_state.get(&group.key).map(Vec::as_slice).unwrap_or(&[]);

    let blank = build_form_state(&group.fields);
    let boxes: Vec<(String, &FormState)> = if instances.is_empty() {
        let count = if group.spans { 2 } else { 1 };
        (0..count)
            .map(|_| (format!("{label}: <span class=\"blank\"></span>"), &blank))
            .collect()
    } else {
        instances
            .iter()
            .enumerate()
            .map(|(i, inst)| {
                let title = match &inst.source_item {
                    Some(item) => format!("{label}: {}", esc(item)),
                    None => format!("{label} {}", i + 1),
                };
                (title, &inst.values)
            })
            .collect()
    };

    let buttons: Vec<&SpeedButtonDef> = group.speed_buttons.iter().collect();
    for (title, values) in boxes {
        let _ = write!(out, "<div class=\"box\"><div class=\"bh\">{title}</div>");
        if !buttons.is_empty() {
            out.push_str("<div class=\"btns\">");
            buttons_html(out, &buttons);
            out.push_str("</div>");
        }
        for field in &group.fields {
            field_html(out, field, &group.fields, values);
        }
        out.push_str("</div>");
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// One US Letter sheet, front and back: two dense columns of small type.
pub const STYLE: &str = "\
@page { size: letter; margin: 0.4in; }
* { box-sizing: border-box; -webkit-print-color-adjust: exact; print-color-adjust: exact; }
body { margin: 0; background: #fff; color: #000; font: 8.5pt/1.3 -apple-system, 'Helvetica Neue', Arial, sans-serif; }
header { display: flex; justify-content: space-between; align-items: baseline; border-bottom: 1.5px solid #000; }
h1 { font-size: 12pt; margin: 0; }
.legend { font-size: 7.5pt; color: #444; margin: 2px 0 4px; }
.presets { border-bottom: 1px solid #000; padding: 2px 0 4px; margin-bottom: 4px; }
main { column-count: 2; column-gap: 0.3in; }
h2 { font-size: 9pt; text-transform: uppercase; letter-spacing: 0.03em; margin: 6px 0 2px; border-bottom: 1px solid #000; break-after: avoid; }
.f { margin: 0 0 2px; break-inside: avoid; }
.f.cond { margin-left: 1.4em; }
.l { color: #444; margin-right: 0.3em; }
.o { white-space: nowrap; margin-right: 0.6em; }
.def { font-weight: 700; text-decoration: underline; }
.blank { display: inline-block; min-width: 9em; min-height: 1.2em; border-bottom: 1px solid #888; }
.line { min-height: 1.5em; border-bottom: 1px solid #888; }
.btns { margin: 1px 0 3px; }
.tick { padding-left: 1.2em; text-indent: -1.2em; margin-bottom: 1px; }
.box { border: 1px solid #000; padding: 2px 5px; margin: 3px 0; break-inside: avoid; }
.bh { font-weight: 700; margin-bottom: 1px; }
.bh .blank { min-width: 5em; }
.chart table { border-collapse: separate; border-spacing: 1px; margin: 1px 0 3px; }
.chart td { width: 1.7em; height: 1.7em; text-align: center; font-size: 7.5pt; }
.chart td.mid { width: 0.6em; border-left: 1px solid #000; }
.chart td.own { border: 1.5px solid #000; border-radius: 50%; font-weight: 700; }
.chart td.lnk { border: 1.5px solid #000; font-weight: 700; }
.notes .line { min-height: 1.7em; }
";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::{build_groups_state, sync_source_group};
    use crate::template::OptionDef;

    fn field(key: &str, ty: FieldType, options: &[&str]) -> FieldDef {
        FieldDef {
            key: key.to_string(),
            label: key.to_string(),
            field_type: ty,
            options: options
                .iter()
                .map(|o| OptionDef { label: o.to_string(), text: None })
                .collect(),
            ..Default::default()
        }
    }

    fn template(fields: Vec<FieldDef>, groups: Vec<GroupDef>) -> TemplateDef {
        TemplateDef {
            id: "t".to_string(),
            name: "Consult".to_string(),
            description: None,
            fields,
            groups,
            speed_buttons: vec![],
            print_skip: vec![],
            body: String::new(),
        }
    }

    fn sheet(t: &TemplateDef, state: &FormState, groups_state: &GroupsState) -> String {
        sheet_html(t, state, groups_state, jiff::civil::date(2026, 9, 28))
    }

    /// The sheet for `t` with every field at its default and no instances.
    fn default_sheet(t: &TemplateDef) -> String {
        sheet(t, &build_form_state(&t.fields), &build_groups_state(&t.groups))
    }

    #[test]
    fn a_default_prints_bold_and_unmarked_and_a_changed_value_prints_filled() {
        let mut hygiene = field("hygiene", FieldType::Dropdown, &["good", "fair", "poor"]);
        hygiene.default = Some(toml::Value::String("good".into()));
        let t = template(vec![hygiene], vec![]);

        let html = default_sheet(&t);
        assert!(html.contains("<span class=\"o def\">&#9675; good</span>"), "{html}");
        assert!(html.contains("<span class=\"o\">&#9675; fair</span>"));
        assert!(!html.contains("&#9679;"), "nothing is filled while the default stands");

        let mut state = build_form_state(&t.fields);
        state.insert("hygiene".into(), FieldValue::Text("fair".into()));
        let html = sheet(&t, &state, &GroupsState::new());
        assert!(html.contains("<span class=\"o\">&#9679; fair</span>"), "{html}");
        assert!(html.contains("<span class=\"o def\">&#9675; good</span>"), "the default stays bold");
    }

    #[test]
    fn multiselect_uses_squares_and_a_checkbox_prints_as_it_stands() {
        let mut risks = field("risks", FieldType::Multiselect, &["Sinus", "Nerve"]);
        risks.default = Some(toml::Value::Array(vec![]));
        let mut extraoral = field("extraoral", FieldType::Checkbox, &[]);
        extraoral.default = Some(toml::Value::Boolean(true));
        let referral = field("referral", FieldType::Checkbox, &[]);
        let t = template(vec![risks, extraoral, referral], vec![]);

        let mut state = build_form_state(&t.fields);
        state.insert("risks".into(), FieldValue::MultiSelect(vec!["Nerve".into()]));
        let html = sheet(&t, &state, &GroupsState::new());
        assert!(html.contains("&#9744; Sinus"));
        assert!(html.contains("&#9632; Nerve"));
        assert!(html.contains("<span class=\"o def\">&#9745; extraoral</span>"), "{html}");
        assert!(html.contains("<span class=\"o\">&#9744; referral</span>"), "unticked is not emphasized");
    }

    #[test]
    fn a_conditional_field_prints_indented_even_while_hidden() {
        let perio = field("perio", FieldType::Dropdown, &["Healthy", "Periodontitis"]);
        let mut findings = field("findings", FieldType::Text, &[]);
        findings.visible_if = Some("perio == 'Periodontitis'".into());
        let html = default_sheet(&template(vec![perio, findings], vec![]));
        assert!(html.contains("<div class=\"f cond\"><span class=\"l\">findings</span>"), "{html}");
    }

    #[test]
    fn computed_print_false_and_skipped_sections_stay_off_the_sheet() {
        let mut age_text = field("age_text", FieldType::Computed, &[]);
        age_text.compute = Some("'x'".into());
        let mut hidden = field("hidden_one", FieldType::Text, &[]);
        hidden.print = Some(false);
        let mut hygiene = field("hygiene", FieldType::Text, &[]);
        hygiene.section = Some("Exam".into());
        let mut bone = field("bone_height", FieldType::Text, &[]);
        bone.section = Some("CBCT".into());
        let cbct_group = GroupDef {
            key: "cbct_spans".into(),
            label: "CBCT span".into(),
            section: Some("CBCT".into()),
            fields: vec![field("width", FieldType::Text, &[])],
            ..Default::default()
        };
        let mut t = template(vec![age_text, hidden, hygiene, bone], vec![cbct_group]);
        t.print_skip = vec!["CBCT".into()];
        t.speed_buttons = vec![SpeedButtonDef {
            label: "All adequate".into(),
            section: Some("CBCT".into()),
            ..Default::default()
        }];

        let html = default_sheet(&t);
        assert!(html.contains("<h2>Exam</h2>"));
        for gone in ["age_text", "hidden_one", "CBCT", "bone_height", "CBCT span", "All adequate"] {
            assert!(!html.contains(gone), "'{gone}' should not print:\n{html}");
        }
    }

    #[test]
    fn a_computed_value_marked_print_true_prints_as_tick_boxes() {
        let mut general = field("general", FieldType::Computed, &[]);
        general.label = "General surgical".into();
        general.print = Some(true);
        let mut specific = field("specific", FieldType::Computed, &[]);
        specific.label = "Specific".into();
        specific.print = Some(true);
        let mut empty = field("empty", FieldType::Computed, &[]);
        empty.label = "Nothing yet".into();
        empty.print = Some(true);
        let t = template(vec![general, specific, empty], vec![]);

        let mut state = build_form_state(&t.fields);
        state.insert("general".into(), FieldValue::Text("bleeding, infection".into()));
        state.insert("specific".into(), FieldValue::MultiSelect(vec!["nerve injury".into(), "sinus & graft".into()]));
        let html = sheet(&t, &state, &GroupsState::new());
        assert!(html.contains(
            "<div class=\"l\">General surgical</div><div class=\"tick\">&#9744; bleeding, infection</div>"
        ), "{html}");
        assert!(html.contains(
            "<div class=\"tick\">&#9744; nerve injury</div><div class=\"tick\">&#9744; sinus &amp; graft</div>"
        ));
        assert!(html.contains(
            "<div class=\"l\">Nothing yet</div><div class=\"tick\">&#9744; <span class=\"blank\"></span></div>"
        ));
    }

    #[test]
    fn top_row_buttons_print_as_checkboxes_above_the_form() {
        let mut t = template(vec![field("a", FieldType::Text, &[])], vec![]);
        t.speed_buttons = vec![
            SpeedButtonDef { label: "Implant appropriate".into(), ..Default::default() },
            SpeedButtonDef { label: "Exo & graft".into(), ..Default::default() },
        ];
        let html = default_sheet(&t);
        assert!(html.contains(
            "<div class=\"presets\"><span class=\"o\">&#9744; Implant appropriate</span> \
             <span class=\"o\">&#9744; Exo &amp; graft</span> </div>"
        ), "{html}");
    }

    #[test]
    fn write_ins_leave_the_flag_off_and_take_their_line_count() {
        let mut flagged = field("adjacent", FieldType::Text, &[]);
        flagged.default = Some(toml::Value::String("***".into()));
        let notes = field("notes", FieldType::Textarea, &[]);
        let mut hx = field("hx", FieldType::Textarea, &[]);
        hx.print_lines = Some(2);
        hx.print_label = Some("Other hx".into());
        let html = default_sheet(&template(vec![flagged, notes, hx], vec![]));

        assert!(html.contains("<span class=\"l\">adjacent</span> <span class=\"blank\"></span>"), "{html}");
        assert!(!html.contains("***"));
        let lines_after = |label: &str| {
            let start = html.find(&format!(">{label}</span>")).unwrap();
            let rest = &html[start..];
            rest[..rest.find("</div></div>").unwrap()].matches("class=\"line").count()
        };
        assert_eq!(lines_after("notes"), 3);
        assert_eq!(lines_after("Other hx"), 2);
    }

    #[test]
    fn uncharted_site_groups_print_blank_boxes_and_charted_ones_one_box_each() {
        let missing = field("missing", FieldType::Teeth, &["35", "36", "37", "46"]);
        let spans = GroupDef {
            key: "spans".into(),
            label: "Edentulous span".into(),
            source: Some("missing".into()),
            source_as: Some("span".into()),
            spans: true,
            fields: vec![field("ridge", FieldType::Dropdown, &["Adequate", "Width deficient"])],
            ..Default::default()
        };
        let teeth = GroupDef {
            key: "teeth".into(),
            label: "Tooth".into(),
            source: Some("missing".into()),
            fields: vec![field("cond", FieldType::Multiselect, &["Fractured"])],
            ..Default::default()
        };
        let t = template(vec![missing], vec![spans, teeth]);

        let html = default_sheet(&t);
        assert_eq!(html.matches("Edentulous span: <span class=\"blank\">").count(), 2);
        assert_eq!(html.matches("Tooth: <span class=\"blank\">").count(), 1);

        let mut state = build_form_state(&t.fields);
        state.insert("missing".into(), FieldValue::MultiSelect(vec!["35".into(), "36".into(), "46".into()]));
        let mut groups_state = build_groups_state(&t.groups);
        for g in &t.groups {
            sync_source_group(g, &state, &mut groups_state);
        }
        let html = sheet(&t, &state, &groups_state);
        assert!(html.contains("Edentulous span: 46</div>"), "{html}");
        assert!(html.contains("Edentulous span: 35-36</div>"));
        assert_eq!(html.matches("<div class=\"box\">").count(), 5, "2 spans + 3 teeth");
    }

    #[test]
    fn a_linked_chart_marks_each_list_its_own_way() {
        let mut missing = field("missing", FieldType::Teeth, &[]);
        missing.label = "Edentulous".into();
        missing.linked = Some("present".into());
        let mut present = field("present", FieldType::Teeth, &[]);
        present.label = "Tooth present".into();
        let mut t = template(vec![missing, present], vec![]);
        for f in &mut t.fields {
            f.options = crate::template::FDI_TEETH
                .iter()
                .map(|n| OptionDef { label: n.to_string(), text: None })
                .collect();
        }
        let mut state = build_form_state(&t.fields);
        state.insert("missing".into(), FieldValue::MultiSelect(vec!["36".into()]));
        state.insert("present".into(), FieldValue::MultiSelect(vec!["46".into()]));
        let html = sheet(&t, &state, &GroupsState::new());

        assert!(html.contains("Circle = Edentulous; box = Tooth present"), "{html}");
        assert!(html.contains("<td class=\"own\">36</td>"));
        assert!(html.contains("<td class=\"lnk\">46</td>"));
        assert_eq!(html.matches("<table>").count(), 1, "the linked list shares the chart");
        assert!(!html.contains(">55<"), "primary rows stay off until a primary tooth is charted");
    }

    #[test]
    fn every_bundled_template_prints() {
        for dir in ["my_templates", "examples/templates"] {
            let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
            let result = crate::template::loader::load_templates(&dir);
            assert!(result.errors.is_empty(), "{:?}", result.errors);
            for t in &result.templates {
                let html = default_sheet(t);
                assert!(html.contains(&format!("<h1>{}</h1>", esc(&t.name))), "{}", t.id);
                assert!(html.ends_with("</html>"));
            }
        }
    }
}
