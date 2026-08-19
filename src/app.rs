use eframe::egui;

use crate::drafts::DraftEntry;
use crate::field::{
    add_group_instance, apply_speed_button, build_form_state, build_groups_state, is_restorable,
    snapshot_from_json, snapshot_to_json, FieldValue, FormState, GroupsState,
};
use crate::formatting::{self, FormattedRun};
use crate::history::{append_history, load_history, now_timestamp, HistoryEntry};
use crate::render::render_body;
use crate::storage::Paths;
use crate::template::loader::load_templates;
use crate::template::{FieldDef, FieldType, GroupDef, TemplateDef};

pub struct NoteTemplaterApp {
    paths: Paths,
    templates: Vec<TemplateDef>,
    partials: Vec<(String, String)>,
    load_errors: Vec<(String, String)>,
    selected: Option<usize>,
    form_state: FormState,
    groups_state: GroupsState,
    search: String,
    show_history: bool,
    history: Vec<HistoryEntry>,
    show_drafts: bool,
    drafts: Vec<DraftEntry>,
    status: Option<String>,
}

impl NoteTemplaterApp {
    pub fn new(paths: Paths) -> Self {
        let result = load_templates(&paths.templates_dir);
        let history = load_history(&paths.history_file);
        let drafts = crate::drafts::load_drafts(&paths.drafts_file);

        let mut app = Self {
            paths,
            templates: result.templates,
            partials: result.partials,
            load_errors: result.errors,
            selected: None,
            form_state: FormState::new(),
            groups_state: GroupsState::new(),
            search: String::new(),
            show_history: false,
            history,
            show_drafts: false,
            drafts,
            status: None,
        };
        if !app.templates.is_empty() {
            app.select_template(0);
        }
        app
    }

    fn select_template(&mut self, idx: usize) {
        self.save_draft_for_current();
        if let Some(t) = self.templates.get(idx) {
            self.form_state = build_form_state(&t.fields);
            self.groups_state = build_groups_state(&t.groups);
            self.selected = Some(idx);
            self.status = None;
        }
    }

    fn reload_templates(&mut self) {
        self.save_draft_for_current();
        let result = load_templates(&self.paths.templates_dir);
        self.templates = result.templates;
        self.partials = result.partials;
        self.load_errors = result.errors;
        self.selected = None;
        self.form_state.clear();
        self.groups_state.clear();
        if !self.templates.is_empty() {
            self.select_template(0);
        }
    }

    /// Saves (or clears) the draft for whichever template is currently
    /// selected, using its current form state. Called whenever we're about to
    /// navigate away from it (switching templates, reloading, exiting) so
    /// unsaved progress isn't lost.
    fn save_draft_for_current(&mut self) {
        let Some(idx) = self.selected else { return };
        let Some(template) = self.templates.get(idx) else {
            return;
        };
        let current = snapshot_to_json(
            &template.fields,
            &self.form_state,
            &template.groups,
            &self.groups_state,
        );
        let defaults = snapshot_to_json(
            &template.fields,
            &build_form_state(&template.fields),
            &template.groups,
            &build_groups_state(&template.groups),
        );
        let has_progress = current != defaults;
        crate::drafts::upsert(
            &self.paths.drafts_file,
            &mut self.drafts,
            &template.id,
            &template.name,
            current,
            has_progress,
        );
    }

    /// Whether a saved (template_id, field_values) pair — from a history entry
    /// or a draft — can actually be restored into the form: the template must
    /// still exist, and its fields must still overlap with what was saved.
    /// False for entries saved before the restore feature existed (no
    /// field_values at all) or for a template whose fields changed enough
    /// since that none of the saved keys still apply.
    fn can_restore(&self, template_id: &str, field_values: &serde_json::Value) -> bool {
        self.templates
            .iter()
            .find(|t| t.id == template_id)
            .is_some_and(|t| is_restorable(&t.fields, field_values))
    }

    /// Loads a template + its saved field values (from a history entry or
    /// draft) into the form. Returns `false` (with a status message) if the
    /// template no longer exists.
    fn load_saved_state(
        &mut self,
        template_id: &str,
        template_name: &str,
        field_values: &serde_json::Value,
    ) -> bool {
        let Some(idx) = self.templates.iter().position(|t| t.id == template_id) else {
            self.status = Some(format!("Template \"{template_name}\" no longer exists."));
            return false;
        };
        self.save_draft_for_current();
        let template = &self.templates[idx];
        let (state, groups_state) = snapshot_from_json(&template.fields, &template.groups, field_values);
        self.form_state = state;
        self.groups_state = groups_state;
        self.selected = Some(idx);
        true
    }

    fn restore_from_history(&mut self, index: usize) {
        let Some(entry) = self.history.get(index).cloned() else {
            return;
        };
        if self.load_saved_state(&entry.template_id, &entry.template_name, &entry.field_values) {
            self.show_history = false;
            self.status = Some(format!("Loaded \"{}\" from history.", entry.template_name));
        }
    }

    fn restore_from_draft(&mut self, index: usize) {
        let Some(draft) = self.drafts.get(index).cloned() else {
            return;
        };
        if self.load_saved_state(&draft.template_id, &draft.template_name, &draft.field_values) {
            self.show_drafts = false;
            self.status = Some(format!("Resumed draft of \"{}\".", draft.template_name));
        }
    }

    fn discard_draft(&mut self, index: usize) {
        if index < self.drafts.len() {
            self.drafts.remove(index);
            let _ = crate::drafts::save_drafts(&self.paths.drafts_file, &self.drafts);
        }
    }

    fn form_is_valid(&self, template: &TemplateDef) -> bool {
        fields_are_valid(&template.fields, &self.form_state)
            && template.groups.iter().all(|group| {
                self.groups_state
                    .get(&group.key)
                    .is_none_or(|instances| {
                        instances
                            .iter()
                            .all(|inst| fields_are_valid(&group.fields, &inst.values))
                    })
            })
    }
}

/// Whether every field in `fields` currently holds a value that would let a
/// Copy go ahead: required fields aren't empty, and no date field (required
/// or not) has typed-but-unparsable text sitting in it. Shared by the
/// top-level form and, per-instance, by every repeatable group.
fn fields_are_valid(fields: &[FieldDef], state: &FormState) -> bool {
    fields.iter().all(|f| {
        let Some(value) = state.get(&f.key) else {
            return !f.required;
        };
        if f.required && value.is_empty() {
            return false;
        }
        // Even for an optional date field, typed-but-unparsable text blocks
        // copying — better to force a fix than silently copy a wrong date.
        if let FieldValue::Date(d) = value
            && d.is_invalid()
        {
            return false;
        }
        true
    })
}

impl eframe::App for NoteTemplaterApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("top_bar").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.heading("Note Templater");
                if ui.button("Reload Templates").clicked() {
                    self.reload_templates();
                }
                ui.toggle_value(&mut self.show_history, "History");
                ui.toggle_value(&mut self.show_drafts, "Drafts");
                if !self.load_errors.is_empty() {
                    ui.colored_label(
                        egui::Color32::from_rgb(200, 60, 60),
                        format!("{} template(s) failed to load", self.load_errors.len()),
                    );
                }
            });
            if !self.load_errors.is_empty() {
                ui.collapsing("Show load errors", |ui| {
                    for (file, err) in &self.load_errors {
                        ui.label(format!("{file}: {err}"));
                    }
                });
            }
            ui.add_space(4.0);
        });

        if self.show_history {
            let mut restore_index = None;
            let mut copy_again = None;
            egui::Panel::right("history_panel")
                .default_size(320.0)
                .show(ui, |ui| {
                    ui.heading("History");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for i in (0..self.history.len()).rev() {
                            let entry = &self.history[i];
                            let restorable = self.can_restore(&entry.template_id, &entry.field_values);
                            ui.group(|ui| {
                                ui.label(format!("{}  •  {}", entry.timestamp, entry.template_name));
                                let preview: String =
                                    entry.rendered_text.lines().next().unwrap_or("").to_string();
                                ui.label(preview);
                                if !restorable {
                                    ui.label("(fields not available to restore — template is new or has changed)");
                                }
                                ui.horizontal(|ui| {
                                    if restorable && ui.button("Restore").clicked() {
                                        restore_index = Some(i);
                                    }
                                    if ui.button("Copy again").clicked() {
                                        copy_again = Some(entry.rendered_text.clone());
                                    }
                                });
                            });
                        }
                        if self.history.is_empty() {
                            ui.label("No notes copied yet.");
                        }
                    });
                });
            if let Some(i) = restore_index {
                self.restore_from_history(i);
            }
            if let Some(text) = copy_again {
                match copy_to_clipboard(&text) {
                    Ok(()) => self.status = Some("Copied from history!".into()),
                    Err(e) => self.status = Some(format!("Copy failed: {e}")),
                }
            }
        }

        if self.show_drafts {
            let mut restore_index = None;
            let mut discard_index = None;
            egui::Panel::right("drafts_panel")
                .default_size(320.0)
                .show(ui, |ui| {
                    ui.heading("Drafts");
                    ui.label("Unsaved form progress, kept when you switch templates or close the app.");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for i in 0..self.drafts.len() {
                            let draft = &self.drafts[i];
                            let restorable = self.can_restore(&draft.template_id, &draft.field_values);
                            ui.group(|ui| {
                                ui.label(format!("{}  •  {}", draft.updated_at, draft.template_name));
                                if !restorable {
                                    ui.label("(template is new or has changed — can't resume, only discard)");
                                }
                                ui.horizontal(|ui| {
                                    if restorable && ui.button("Resume").clicked() {
                                        restore_index = Some(i);
                                    }
                                    if ui.button("Discard").clicked() {
                                        discard_index = Some(i);
                                    }
                                });
                            });
                        }
                        if self.drafts.is_empty() {
                            ui.label("No drafts.");
                        }
                    });
                });
            if let Some(i) = restore_index {
                self.restore_from_draft(i);
            }
            if let Some(i) = discard_index {
                self.discard_draft(i);
            }
        }

        egui::Panel::left("template_list")
            .default_size(240.0)
            .show(ui, |ui| {
                ui.heading("Templates");
                ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Search…"));
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let search = self.search.to_lowercase();
                    for i in 0..self.templates.len() {
                        let name = self.templates[i].name.clone();
                        if !search.is_empty() && !name.to_lowercase().contains(&search) {
                            continue;
                        }
                        let selected = self.selected == Some(i);
                        if ui.selectable_label(selected, &name).clicked() {
                            self.select_template(i);
                        }
                    }
                });
            });

        egui::CentralPanel::default_margins().show(ui, |ui| {
            if self.templates.is_empty() {
                ui.label(
                    "No templates found. Add .toml files to the templates directory and click \"Reload Templates\".",
                );
                return;
            }
            let Some(idx) = self.selected else {
                ui.label("Select a template from the left.");
                return;
            };
            // Cloned so we don't hold an immutable borrow of `self.templates`
            // while mutating `self.form_state` below.
            let template = self.templates[idx].clone();

            // A resizable inner panel for the form, so the preview (drawn directly
            // into the remaining `ui` below) can be made bigger or smaller by
            // dragging the divider. The form panel is shown first so its widgets
            // update `self.form_state` before the preview reads it this same frame.
            egui::Panel::left("form_panel")
                .resizable(true)
                .default_size(480.0)
                .min_size(280.0)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("form_scroll")
                        .show(ui, |ui| {
                            ui.heading(&template.name);
                            if let Some(desc) = &template.description {
                                ui.label(desc);
                            }
                            if !template.speed_buttons.is_empty() {
                                ui.add_space(4.0);
                                ui.horizontal_wrapped(|ui| {
                                    for sb in &template.speed_buttons {
                                        if ui.button(&sb.label).clicked() {
                                            apply_speed_button(
                                                sb,
                                                &template.fields,
                                                &mut self.form_state,
                                                &template.groups,
                                                &mut self.groups_state,
                                            );
                                        }
                                    }
                                });
                            }
                            ui.separator();
                            for field in &template.fields {
                                render_field(ui, field, &mut self.form_state);
                            }

                            for group in &template.groups {
                                render_group(ui, group, &mut self.groups_state);
                            }
                        });
                });

            let rendered = render_body(
                &template.body,
                &self.partials,
                &self.form_state,
                &template.groups,
                &self.groups_state,
            );
            // `CentralPanel` (rather than `ui.vertical`) so this fills all remaining
            // width instead of shrink-wrapping to its content.
            egui::CentralPanel::default_margins().show(ui, |ui| {
                ui.heading("Preview");
                let runs = rendered.as_ref().ok().map(|text| formatting::parse(text));
                egui::ScrollArea::vertical()
                    .id_salt("preview_scroll")
                    .max_height(ui.available_height() - 60.0)
                    .show(ui, |ui| match &runs {
                        Some(runs) => {
                            let job = build_preview_layout_job(runs, ui);
                            ui.add(egui::Label::new(job).selectable(true));
                        }
                        None => {
                            if let Err(e) = &rendered {
                                ui.colored_label(
                                    egui::Color32::from_rgb(200, 60, 60),
                                    format!("Template error: {e}"),
                                );
                            }
                        }
                    });

                ui.separator();
                ui.horizontal(|ui| {
                    let valid = self.form_is_valid(&template);
                    let can_copy = valid && runs.is_some();
                    let mut copied: Option<String> = None;

                    if ui
                        .add_enabled(can_copy, egui::Button::new("Copy to Clipboard"))
                        .on_hover_text("Copies formatted (bold/italic/underline), with a plain-text fallback for apps that don't support rich paste.")
                        .clicked()
                        && let Some(runs) = &runs
                    {
                        let plain = formatting::to_plain_text(runs);
                        let html = formatting::to_html(runs);
                        match copy_rich_to_clipboard(&html, &plain) {
                            Ok(()) => {
                                self.status = Some("Copied!".to_string());
                                copied = Some(plain);
                            }
                            Err(e) => self.status = Some(format!("Copy failed: {e}")),
                        }
                    }
                    if ui
                        .add_enabled(can_copy, egui::Button::new("Copy Plain Text"))
                        .on_hover_text("Copies with formatting markers stripped — no bold/italic/underline.")
                        .clicked()
                        && let Some(runs) = &runs
                    {
                        let plain = formatting::to_plain_text(runs);
                        match copy_to_clipboard(&plain) {
                            Ok(()) => {
                                self.status = Some("Copied (plain text)!".to_string());
                                copied = Some(plain);
                            }
                            Err(e) => self.status = Some(format!("Copy failed: {e}")),
                        }
                    }

                    if let Some(plain) = copied {
                        let entry = HistoryEntry {
                            timestamp: now_timestamp(),
                            template_id: template.id.clone(),
                            template_name: template.name.clone(),
                            rendered_text: plain,
                            field_values: snapshot_to_json(
                                &template.fields,
                                &self.form_state,
                                &template.groups,
                                &self.groups_state,
                            ),
                        };
                        let _ = append_history(&self.paths.history_file, &entry);
                        self.history.push(entry);
                        // The note is safely in history now, so drop any
                        // in-progress draft for this template.
                        crate::drafts::upsert(
                            &self.paths.drafts_file,
                            &mut self.drafts,
                            &template.id,
                            &template.name,
                            serde_json::Value::Null,
                            false,
                        );
                    }

                    if !valid {
                        ui.label("Fill required fields and fix invalid dates to enable copy.");
                    }
                    if let Some(status) = &self.status {
                        ui.label(status);
                    }
                });
            });
        });
    }

    fn on_exit(&mut self) {
        self.save_draft_for_current();
    }
}

fn render_field(ui: &mut egui::Ui, field: &FieldDef, state: &mut FormState) {
    ui.horizontal(|ui| {
        ui.label(&field.label);
        if field.required {
            ui.colored_label(egui::Color32::from_rgb(200, 60, 60), "*");
        }
    });
    match field.field_type {
        FieldType::Text => {
            if let Some(FieldValue::Text(s)) = state.get_mut(&field.key) {
                ui.add(egui::TextEdit::singleline(s));
            }
        }
        FieldType::Textarea => {
            if let Some(FieldValue::Text(s)) = state.get_mut(&field.key) {
                ui.add(egui::TextEdit::multiline(s).desired_rows(3));
            }
        }
        FieldType::Number => {
            if let Some(FieldValue::Number(n)) = state.get_mut(&field.key) {
                let mut drag = egui::DragValue::new(n);
                if let (Some(min), Some(max)) = (field.min, field.max) {
                    drag = drag.range(min..=max);
                }
                ui.add(drag);
            }
        }
        FieldType::Checkbox => {
            if let Some(FieldValue::Bool(b)) = state.get_mut(&field.key) {
                ui.checkbox(b, "");
            }
        }
        FieldType::Dropdown => {
            if let Some(FieldValue::Text(s)) = state.get_mut(&field.key) {
                let current = s.clone();
                egui::ComboBox::from_id_salt(&field.key)
                    .selected_text(current)
                    .show_ui(ui, |ui| {
                        for opt in &field.options {
                            ui.selectable_value(s, opt.clone(), opt);
                        }
                    });
            }
        }
        FieldType::Multiselect => {
            if let Some(FieldValue::MultiSelect(selected)) = state.get_mut(&field.key) {
                for opt in &field.options {
                    let mut checked = selected.contains(opt);
                    if ui.checkbox(&mut checked, opt).changed() {
                        if checked {
                            selected.push(opt.clone());
                        } else {
                            selected.retain(|o| o != opt);
                        }
                    }
                }
            }
        }
        FieldType::Date => {
            if let Some(FieldValue::Date(d)) = state.get_mut(&field.key) {
                ui.horizontal(|ui| {
                    let invalid = d.is_invalid();
                    let mut text_edit =
                        egui::TextEdit::singleline(&mut d.text).desired_width(110.0);
                    if invalid {
                        text_edit = text_edit.text_color(egui::Color32::from_rgb(200, 60, 60));
                    }
                    if ui.add(text_edit).changed() {
                        d.parsed = d.text.trim().parse().ok();
                    }

                    // The picker button needs a plain `&mut Date` to seed/read its
                    // popup; it doesn't have anywhere to represent "invalid", so we
                    // fall back to today's date while the typed text is unparsable.
                    let mut picker_date = d.parsed.unwrap_or_else(crate::field::today);
                    if ui
                        .add(egui_extras::DatePickerButton::new(&mut picker_date).id_salt(&field.key))
                        .changed()
                    {
                        d.parsed = Some(picker_date);
                        d.text = picker_date.to_string();
                    }

                    if invalid {
                        ui.colored_label(
                            egui::Color32::from_rgb(200, 60, 60),
                            "Invalid date (YYYY-MM-DD)",
                        );
                    }
                });
            }
        }
    }
    ui.add_space(8.0);
}

/// Renders one repeatable group as a list of add/remove-able instances, each
/// a mini form using the same [`render_field`] as the top-level fields.
fn render_group(ui: &mut egui::Ui, group: &GroupDef, groups_state: &mut GroupsState) {
    ui.add_space(4.0);
    ui.separator();
    ui.strong(&group.label);

    let instances = groups_state.entry(group.key.clone()).or_default();
    let mut remove_index = None;
    for (i, instance) in instances.iter_mut().enumerate() {
        ui.push_id(instance.id, |ui| {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("{} {}", group.label, i + 1));
                    if ui.small_button("Remove").clicked() {
                        remove_index = Some(i);
                    }
                });
                for field in &group.fields {
                    render_field(ui, field, &mut instance.values);
                }
            });
        });
    }
    if let Some(i) = remove_index {
        instances.remove(i);
    }

    if ui.button(format!("+ Add {}", group.label)).clicked() {
        add_group_instance(groups_state, group);
    }
    ui.add_space(4.0);
}

fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(text.to_string()).map_err(|e| e.to_string())
}

/// Puts both `html` and `plain` on the clipboard in one go — a rich-text
/// target (Word, Notes, Mail, ...) picks up the HTML and its bold/italic/
/// underline; anything that only understands plain text falls back to `plain`.
fn copy_rich_to_clipboard(html: &str, plain: &str) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard
        .set_html(html, Some(plain))
        .map_err(|e| e.to_string())
}

/// Builds the preview's rich-text layout from parsed formatting runs. Bold
/// renders as the theme's "strong" color rather than a heavier font weight —
/// egui doesn't bundle a bold font face — while italic and underline use
/// egui's native support. The copied HTML still gets *real* bold when pasted
/// into an app with its own bold font.
fn build_preview_layout_job(runs: &[FormattedRun], ui: &egui::Ui) -> egui::text::LayoutJob {
    let font_id = egui::TextStyle::Body.resolve(ui.style());
    let visuals = ui.visuals();
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = ui.available_width();
    for run in runs {
        let color = if run.bold {
            visuals.strong_text_color()
        } else {
            visuals.text_color()
        };
        let mut format = egui::TextFormat {
            font_id: font_id.clone(),
            color,
            italics: run.italic,
            ..Default::default()
        };
        if run.underline {
            format.underline = egui::Stroke::new(1.0, color);
        }
        job.append(&run.text, 0.0, format);
    }
    job
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::add_group_instance;

    fn dummy_paths() -> Paths {
        let dir = std::env::temp_dir();
        Paths {
            templates_dir: dir.clone(),
            history_file: dir.join("note_templater_test_history.jsonl"),
            drafts_file: dir.join("note_templater_test_drafts.json"),
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

    /// Builds an app with no on-disk state loaded — `form_is_valid` never
    /// touches `paths`/history/drafts, so this is safe to construct directly
    /// for a unit test without going through `NoteTemplaterApp::new`.
    fn app_with_template(template: TemplateDef) -> (NoteTemplaterApp, TemplateDef) {
        let app = NoteTemplaterApp {
            paths: dummy_paths(),
            templates: vec![template.clone()],
            partials: Vec::new(),
            load_errors: Vec::new(),
            selected: Some(0),
            form_state: build_form_state(&template.fields),
            groups_state: build_groups_state(&template.groups),
            search: String::new(),
            show_history: false,
            history: Vec::new(),
            show_drafts: false,
            drafts: Vec::new(),
            status: None,
        };
        (app, template)
    }

    #[test]
    fn form_is_valid_checks_required_fields_inside_group_instances() {
        let group = GroupDef {
            key: "procedures".to_string(),
            label: "Procedure Step".to_string(),
            fields: vec![text_field("tooth", true)],
        };
        let template = TemplateDef {
            id: "t".to_string(),
            name: "T".to_string(),
            description: None,
            fields: vec![],
            groups: vec![group.clone()],
            speed_buttons: vec![],
            body: String::new(),
        };
        let (mut app, template) = app_with_template(template);

        assert!(
            app.form_is_valid(&template),
            "no instances yet -> nothing to fail"
        );

        add_group_instance(&mut app.groups_state, &group);
        assert!(
            !app.form_is_valid(&template),
            "a fresh instance has its required 'tooth' field blank"
        );

        let instances = app.groups_state.get_mut("procedures").unwrap();
        instances[0]
            .values
            .insert("tooth".to_string(), FieldValue::Text("14".to_string()));
        assert!(app.form_is_valid(&template), "filled -> valid again");

        add_group_instance(&mut app.groups_state, &group);
        assert!(
            !app.form_is_valid(&template),
            "one bad instance among several still blocks copy"
        );
    }

    #[test]
    fn form_is_valid_ignores_optional_group_fields_left_blank() {
        let group = GroupDef {
            key: "procedures".to_string(),
            label: "Procedure Step".to_string(),
            fields: vec![text_field("notes", false)],
        };
        let template = TemplateDef {
            id: "t".to_string(),
            name: "T".to_string(),
            description: None,
            fields: vec![],
            groups: vec![group.clone()],
            speed_buttons: vec![],
            body: String::new(),
        };
        let (mut app, template) = app_with_template(template);

        add_group_instance(&mut app.groups_state, &group);
        assert!(
            app.form_is_valid(&template),
            "optional field left blank shouldn't block copy"
        );
    }
}
