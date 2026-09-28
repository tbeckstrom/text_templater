use eframe::egui;

use crate::drafts::DraftEntry;
use crate::field::{
    add_group_instance, apply_speed_button, apply_values, build_form_state, build_groups_state,
    is_restorable, snapshot_from_json, snapshot_to_json, sync_source_group, toggle_multiselect,
    FieldValue, FormState, GroupsState,
};
use crate::formatting::{self, FormattedRun};
use crate::history::{append_history, load_history, now_timestamp, HistoryEntry};
use crate::render::{build_context, instance_context_json, render_body_with_context};
use crate::storage::Paths;
use crate::template::loader::load_templates;
use crate::template::{FieldDef, FieldType, GroupDef, PartialDef, SpeedButtonDef, TemplateDef};
use crate::visibility::{
    context_with_overrides, recompute_fields, ExprEvaluator, Scope, Visibility,
};
use tera::Context;

pub struct NoteTemplaterApp {
    paths: Paths,
    templates: Vec<TemplateDef>,
    partials: Vec<PartialDef>,
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
    /// Caches compiled `visible_if` expressions across frames.
    expr_eval: ExprEvaluator,
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
            expr_eval: ExprEvaluator::default(),
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

    fn form_is_valid(&self, template: &TemplateDef, visibility: &Visibility) -> bool {
        fields_are_valid(&template.fields, &self.form_state, visibility, Scope::Form)
            && template.groups.iter().all(|group| {
                if !visibility.is_visible(group.visible_if.as_ref(), Scope::Form) {
                    return true;
                }
                self.groups_state
                    .get(&group.key)
                    .is_none_or(|instances| {
                        instances.iter().all(|inst| {
                            fields_are_valid(
                                &group.fields,
                                &inst.values,
                                visibility,
                                Scope::Instance(inst.id),
                            )
                        })
                    })
            })
    }

    /// Works out, once per frame, which `visible_if`-guarded fields, groups
    /// and speed buttons are currently showing. Conditions inside a
    /// repeatable block are evaluated per instance, with that instance's own
    /// values layered over the template's top-level ones.
    fn compute_visibility(&mut self, template: &TemplateDef, ctx: &Context) -> Visibility {
        let mut visibility = Visibility::default();
        let eval = |visibility: &mut Visibility,
                        evaluator: &mut ExprEvaluator,
                        expr: &Option<String>,
                        scope: Scope,
                        ctx: &Context| {
            if let Some(expr) = expr {
                let shown = evaluator.truthy(expr, ctx);
                visibility.record(expr, scope, shown);
            }
        };

        for field in &template.fields {
            eval(
                &mut visibility,
                &mut self.expr_eval,
                &field.visible_if,
                Scope::Form,
                ctx,
            );
        }
        for sb in &template.speed_buttons {
            eval(
                &mut visibility,
                &mut self.expr_eval,
                &sb.visible_if,
                Scope::Form,
                ctx,
            );
        }
        for group in &template.groups {
            eval(
                &mut visibility,
                &mut self.expr_eval,
                &group.visible_if,
                Scope::Form,
                ctx,
            );
            let Some(instances) = self.groups_state.get(&group.key) else {
                continue;
            };
            for inst in instances {
                let needs_scope = group.fields.iter().any(|f| f.visible_if.is_some())
                    || group.speed_buttons.iter().any(|sb| sb.visible_if.is_some());
                if !needs_scope {
                    continue;
                }
                let inst_ctx = context_with_overrides(ctx, &instance_context_json(group, inst));
                let scope = Scope::Instance(inst.id);
                for field in &group.fields {
                    eval(
                        &mut visibility,
                        &mut self.expr_eval,
                        &field.visible_if,
                        scope,
                        &inst_ctx,
                    );
                }
                for sb in &group.speed_buttons {
                    eval(
                        &mut visibility,
                        &mut self.expr_eval,
                        &sb.visible_if,
                        scope,
                        &inst_ctx,
                    );
                }
            }
        }
        visibility
    }
}

/// Whether every field in `fields` currently holds a value that would let a
/// Copy go ahead: required fields aren't empty, and no date field (required
/// or not) has typed-but-unparsable text sitting in it. Shared by the
/// top-level form and, per-instance, by every repeatable group.
///
/// A field hidden by `visible_if` is skipped entirely — something the form
/// isn't showing must not be able to block the Copy button.
fn fields_are_valid(
    fields: &[FieldDef],
    state: &FormState,
    visibility: &Visibility,
    scope: Scope,
) -> bool {
    fields.iter().all(|f| {
        if !visibility.is_visible(f.visible_if.as_ref(), scope) {
            return true;
        }
        // Nothing the user can do about a computed value, so it never blocks.
        if f.field_type == FieldType::Computed {
            return true;
        }
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
        // Keyboard shortcuts, read before anything draws so a widget that
        // consumes the key can't swallow them. `/` is only a shortcut when no
        // text field has focus, otherwise it's just a slash being typed.
        let (focus_search, copy_requested) = ui.ctx().input_mut(|i| {
            let copy = i.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::COMMAND,
                egui::Key::Enter,
            ));
            let slash = i.key_pressed(egui::Key::Slash);
            (slash, copy)
        });
        let focus_search = focus_search && ui.ctx().memory(|m| m.focused().is_none());

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
                let search_box =
                    ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Search…  ( / )"));
                if focus_search {
                    search_box.request_focus();
                }
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

            // Bring any selection-driven groups in line with their driving
            // field before drawing, so the form shows one block per choice.
            for group in &template.groups {
                sync_source_group(group, &self.form_state, &mut self.groups_state);
            }
            // Derived values are refreshed first, so `visible_if` and the note
            // body both see this frame's computed results.
            {
                let mut ctx = build_context(
                    &template.fields,
                    &self.form_state,
                    &template.groups,
                    &self.groups_state,
                );
                recompute_fields(
                    &template.fields,
                    &mut self.form_state,
                    &mut ctx,
                    &mut self.expr_eval,
                );
                for group in &template.groups {
                    let Some(instances) = self.groups_state.get_mut(&group.key) else {
                        continue;
                    };
                    for inst in instances.iter_mut() {
                        let mut inst_ctx = ctx.clone();
                        recompute_fields(
                            &group.fields,
                            &mut inst.values,
                            &mut inst_ctx,
                            &mut self.expr_eval,
                        );
                    }
                }
            }

            // Visibility is resolved once, against the values as they stand at
            // the start of the frame, and then just looked up while drawing.
            let visibility = {
                let ctx = build_context(
                    &template.fields,
                    &self.form_state,
                    &template.groups,
                    &self.groups_state,
                );
                self.compute_visibility(&template, &ctx)
            };

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
                            let top_row: Vec<&SpeedButtonDef> = template
                                .speed_buttons
                                .iter()
                                .filter(|sb| {
                                    !is_field_section(&template.fields, sb.section.as_deref())
                                })
                                .collect();
                            if !top_row.is_empty() {
                                ui.add_space(4.0);
                                ui.horizontal_wrapped(|ui| {
                                    for sb in top_row {
                                        if !visibility
                                            .is_visible(sb.visible_if.as_ref(), Scope::Form)
                                        {
                                            continue;
                                        }
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
                            let clicked = render_fields_in_sections(
                                ui,
                                &template.fields,
                                &mut self.form_state,
                                &visibility,
                                &template.speed_buttons,
                                &template.groups,
                                &mut self.groups_state,
                            );
                            if let Some(index) = clicked {
                                apply_speed_button(
                                    &template.speed_buttons[index],
                                    &template.fields,
                                    &mut self.form_state,
                                    &template.groups,
                                    &mut self.groups_state,
                                );
                            }

                            for group in template.groups.iter().filter(|g| {
                                !is_field_section(&template.fields, g.section.as_deref())
                            }) {
                                render_group(ui, group, &mut self.groups_state, &visibility);
                            }
                        });
                });

            let rendered = {
                let ctx = build_context(
                    &template.fields,
                    &self.form_state,
                    &template.groups,
                    &self.groups_state,
                );
                render_body_with_context(&template.body, &self.partials, &ctx)
            };
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
                    let valid = self.form_is_valid(&template, &visibility);
                    let can_copy = valid && runs.is_some();
                    let mut copied: Option<String> = None;

                    let copy_clicked = ui
                        .add_enabled(can_copy, egui::Button::new("Copy to Clipboard  (⌘⏎)"))
                        .on_hover_text("Copies formatted (bold/italic/underline), with a plain-text fallback for apps that don't support rich paste.")
                        .clicked();
                    if let Some(runs) = &runs
                        && can_copy
                        && (copy_clicked || copy_requested)
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

/// A dropdown with at most this many choices is drawn as a row of chips.
const CHIP_LIMIT: usize = 10;

/// A multiselect whose labels are all at most this long lays its checkboxes
/// out side by side instead of one per line.
const COMPACT_LABEL_CHARS: usize = 24;

/// FDI numbers as they sit on a chart facing the patient: the patient's right
/// on the viewer's left, upper arch over lower, with the midline between
/// the two halves of each row.
const PERMANENT_CHART: [[&str; 16]; 2] = [
    ["18", "17", "16", "15", "14", "13", "12", "11", "21", "22", "23", "24", "25", "26", "27", "28"],
    ["48", "47", "46", "45", "44", "43", "42", "41", "31", "32", "33", "34", "35", "36", "37", "38"],
];
const PRIMARY_CHART: [[&str; 10]; 2] = [
    ["55", "54", "53", "52", "51", "61", "62", "63", "64", "65"],
    ["85", "84", "83", "82", "81", "71", "72", "73", "74", "75"],
];

/// Colour of a tooth in a `teeth` field's own list, and in its linked list.
const OWN_TOOTH: egui::Color32 = egui::Color32::from_rgb(70, 130, 200);
const LINKED_TOOTH: egui::Color32 = egui::Color32::from_rgb(214, 140, 40);

const TOOTH_BUTTON: egui::Vec2 = egui::vec2(26.0, 20.0);
const TOOTH_SPACING: f32 = 2.0;
const MIDLINE_GAP: f32 = 10.0;

/// A `teeth` field as an FDI chart: click a tooth, or press and drag across
/// several, to paint them; the Tooth and Quadrant buttons paint whole sets.
/// With `linked`, the chart also holds that field's teeth, and a paint mode
/// picks which list strokes go into (a tooth is only ever in one).
fn odontogram(ui: &mut egui::Ui, field: &FieldDef, siblings: &[FieldDef], state: &mut FormState) {
    let linked = field
        .linked
        .as_ref()
        .and_then(|key| siblings.iter().find(|f| &f.key == key));

    let mode_id = ui.make_persistent_id(("teeth_mode", &field.key));
    let mut painting_linked = ui.data(|d| d.get_temp::<bool>(mode_id)).unwrap_or(false);
    if let Some(other) = linked {
        ui.horizontal(|ui| {
            ui.label("Paint:");
            for (is_linked, label, color) in
                [(false, &field.label, OWN_TOOTH), (true, &other.label, LINKED_TOOTH)]
            {
                ui.colored_label(color, "■");
                if ui.selectable_label(painting_linked == is_linked, label).clicked() {
                    painting_linked = is_linked;
                }
            }
        });
        ui.data_mut(|d| d.insert_temp(mode_id, painting_linked));
    }

    let mut chart = Chart {
        field,
        own: take_list(state, &field.key),
        linked: linked.map(|o| take_list(state, &o.key)).unwrap_or_default(),
        painting_linked: painting_linked && linked.is_some(),
    };
    chart.draw(ui);
    state.insert(field.key.clone(), FieldValue::MultiSelect(chart.own));
    if let Some(other) = linked {
        state.insert(other.key.clone(), FieldValue::MultiSelect(chart.linked));
    }
}

fn take_list(state: &FormState, key: &str) -> Vec<String> {
    match state.get(key) {
        Some(FieldValue::MultiSelect(v)) => v.clone(),
        _ => Vec::new(),
    }
}

/// The two tooth lists one odontogram edits, and which one is being painted.
struct Chart<'a> {
    field: &'a FieldDef,
    own: Vec<String>,
    linked: Vec<String>,
    painting_linked: bool,
}

impl Chart<'_> {
    fn offered(&self, tooth: &str) -> bool {
        self.field.options.iter().any(|o| o.label == tooth)
    }

    fn painted(&self, tooth: &str) -> bool {
        let list = if self.painting_linked { &self.linked } else { &self.own };
        list.iter().any(|t| t == tooth)
    }

    /// Adds `tooth` to the list being painted (taking it out of the other),
    /// or removes it from that list.
    fn paint(&mut self, tooth: &str, on: bool) {
        let (target, other) = if self.painting_linked {
            (&mut self.linked, &mut self.own)
        } else {
            (&mut self.own, &mut self.linked)
        };
        toggle_multiselect(self.field, target, tooth, on);
        if on {
            toggle_multiselect(self.field, other, tooth, false);
        }
    }

    /// Paints every offered tooth in `teeth`, or clears them if all of them
    /// are already painted.
    fn paint_set(&mut self, teeth: &[String]) {
        let teeth: Vec<&String> = teeth.iter().filter(|t| self.offered(t)).collect();
        let on = !teeth.iter().all(|t| self.painted(t));
        for tooth in teeth {
            self.paint(tooth, on);
        }
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        // A stroke lasts while the button is held: it paints (or clears, if
        // it started on a painted tooth) every tooth the pointer passes over.
        let drag_id = ui.make_persistent_id(("teeth_drag", &self.field.key));
        if !ui.input(|i| i.pointer.primary_down()) {
            ui.data_mut(|d| d.remove::<bool>(drag_id));
        }

        for teeth in &PERMANENT_CHART {
            self.row(ui, teeth, 0.0, drag_id);
        }
        let primary_id = ui.make_persistent_id(("show_primary_teeth", &self.field.key));
        let any_primary = self
            .own
            .iter()
            .chain(&self.linked)
            .any(|t| crate::template::is_primary_tooth(t));
        let mut show_primary = ui.data(|d| d.get_temp::<bool>(primary_id)).unwrap_or(false) || any_primary;
        if show_primary {
            // Centred under the permanent arch: three tooth widths in.
            let indent = 3.0 * (TOOTH_BUTTON.x + TOOTH_SPACING);
            for teeth in &PRIMARY_CHART {
                self.row(ui, teeth, indent, drag_id);
            }
        }

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = TOOTH_SPACING;
            ui.label("Tooth:");
            for n in 1..=8 {
                if ui.small_button(n.to_string()).on_hover_text(format!("All {n}s")).clicked() {
                    let teeth: Vec<String> = (1..=4).map(|q| format!("{q}{n}")).collect();
                    self.paint_set(&teeth);
                }
            }
            ui.add_space(8.0);
            ui.label("Quadrant:");
            for (q, name) in [(1, "UR"), (2, "UL"), (3, "LL"), (4, "LR")] {
                if ui.small_button(name).clicked() {
                    let teeth: Vec<String> = (1..=8).map(|n| format!("{q}{n}")).collect();
                    self.paint_set(&teeth);
                }
            }
            ui.add_space(8.0);
            if ui.small_button("Clear").clicked() {
                if self.painting_linked {
                    self.linked.clear();
                } else {
                    self.own.clear();
                }
            }
            ui.add_space(8.0);
            if ui.checkbox(&mut show_primary, "Primary").changed() {
                ui.data_mut(|d| d.insert_temp(primary_id, show_primary));
            }
        });
    }

    /// One arch, split at the midline. A tooth the field doesn't offer leaves
    /// a gap so the rest stay in their chart positions.
    fn row(&mut self, ui: &mut egui::Ui, teeth: &[&str], indent: f32, drag_id: egui::Id) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = TOOTH_SPACING;
            ui.add_space(indent);
            let half = teeth.len() / 2;
            for (i, &tooth) in teeth.iter().enumerate() {
                if i == half {
                    ui.add_space(MIDLINE_GAP);
                }
                if !self.offered(tooth) {
                    ui.add_space(TOOTH_BUTTON.x + TOOTH_SPACING);
                    continue;
                }
                let fill = if self.own.iter().any(|t| t == tooth) {
                    OWN_TOOTH
                } else if self.linked.iter().any(|t| t == tooth) {
                    LINKED_TOOTH
                } else {
                    ui.visuals().widgets.inactive.weak_bg_fill
                };
                // Sensing drag keeps the scroll area from claiming the stroke.
                let response = ui.add(
                    egui::Button::new(tooth)
                        .fill(fill)
                        .min_size(TOOTH_BUTTON)
                        .sense(egui::Sense::click_and_drag()),
                );
                let (pressed, down, pos) = ui.input(|i| {
                    (i.pointer.primary_pressed(), i.pointer.primary_down(), i.pointer.interact_pos())
                });
                if !pos.is_some_and(|p| response.rect.contains(p)) {
                    continue;
                }
                if pressed {
                    let on = !self.painted(tooth);
                    ui.data_mut(|d| d.insert_temp(drag_id, on));
                    self.paint(tooth, on);
                } else if down
                    && let Some(on) = ui.data(|d| d.get_temp::<bool>(drag_id))
                    && self.painted(tooth) != on
                {
                    self.paint(tooth, on);
                }
            }
        });
    }
}

fn render_field(
    ui: &mut egui::Ui,
    field: &FieldDef,
    siblings: &[FieldDef],
    state: &mut FormState,
    visibility: &Visibility,
    scope: Scope,
) {
    if !visibility.is_visible(field.visible_if.as_ref(), scope) {
        return;
    }
    // A tooth list linked from another chart is drawn as part of that chart.
    if siblings.iter().any(|f| f.linked.as_deref() == Some(field.key.as_str())) {
        return;
    }
    // A linked chart names both of its lists in its own paint-mode row.
    let labelled_by_chart = field.field_type == FieldType::Teeth && field.linked.is_some();
    ui.horizontal(|ui| {
        if !labelled_by_chart {
            ui.label(&field.label);
        }
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
                if field.options.len() <= CHIP_LIMIT {
                    // Every choice on show as a chip: one click to pick, and
                    // clicking the picked chip again clears it.
                    ui.horizontal_wrapped(|ui| {
                        for label in field.option_labels() {
                            let on = s == label;
                            if ui.selectable_label(on, label).clicked() {
                                *s = if on { String::new() } else { label.to_owned() };
                            }
                        }
                    });
                } else {
                    let current = s.clone();
                    egui::ComboBox::from_id_salt(&field.key)
                        .selected_text(current)
                        .show_ui(ui, |ui| {
                            for label in field.option_labels() {
                                ui.selectable_value(s, label.to_owned(), label);
                            }
                        });
                }
            }
        }
        FieldType::Multiselect => {
            if let Some(FieldValue::MultiSelect(selected)) = state.get_mut(&field.key) {
                let mut checkboxes = |ui: &mut egui::Ui| {
                    for label in field.option_labels() {
                        let mut checked = selected.iter().any(|s| s == label);
                        if ui.checkbox(&mut checked, label).changed() {
                            // Goes through the helper so the stored selection stays
                            // in declared option order, not click order.
                            toggle_multiselect(field, selected, label, checked);
                        }
                    }
                };
                // Short labels sit side by side; sentence-length ones stack.
                let compact = field
                    .option_labels()
                    .all(|l| l.chars().count() <= COMPACT_LABEL_CHARS);
                if compact {
                    ui.horizontal_wrapped(|ui| checkboxes(ui));
                } else {
                    checkboxes(ui);
                }
            }
        }
        FieldType::Teeth => odontogram(ui, field, siblings, state),
        FieldType::Computed => {
            // Read-only: the value comes from `compute`, not from typing.
            let shown = match state.get(&field.key) {
                Some(FieldValue::Text(s)) if !s.is_empty() => s.clone(),
                _ => "—".to_string(),
            };
            ui.add_enabled(false, egui::Label::new(shown));
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

/// Draws `fields` in order, grouping each run of consecutive fields sharing a
/// `section` in one collapsible heading. Fields with no `section` are drawn
/// plainly, so a template that never mentions sections looks exactly as it
/// did before. A section also holds the speed buttons and groups that name
/// it: buttons above its fields, groups after them.
///
/// A section whose fields are all hidden by `visible_if` is skipped entirely
/// rather than left as an empty heading.
///
/// Returns the index (into `buttons`) of a speed button clicked this frame.
fn render_fields_in_sections(
    ui: &mut egui::Ui,
    fields: &[FieldDef],
    state: &mut FormState,
    visibility: &Visibility,
    buttons: &[SpeedButtonDef],
    groups: &[GroupDef],
    groups_state: &mut GroupsState,
) -> Option<usize> {
    let mut clicked = None;
    let mut i = 0;
    while i < fields.len() {
        let section = fields[i].section.clone();
        // How far this run of same-section fields extends.
        let mut end = i;
        while end < fields.len() && fields[end].section == section {
            end += 1;
        }
        let run = &fields[i..end];
        i = end;

        let Some(section) = section else {
            for field in run {
                render_field(ui, field, fields, state, visibility, Scope::Form);
            }
            continue;
        };

        let any_visible = run
            .iter()
            .any(|f| visibility.is_visible(f.visible_if.as_ref(), Scope::Form));
        if !any_visible {
            continue;
        }

        egui::CollapsingHeader::new(&section)
            .id_salt(("section", section.as_str()))
            .default_open(true)
            .show(ui, |ui| {
                let here: Vec<_> = buttons
                    .iter()
                    .enumerate()
                    .filter(|(_, sb)| sb.section.as_deref() == Some(section.as_str()))
                    .filter(|(_, sb)| visibility.is_visible(sb.visible_if.as_ref(), Scope::Form))
                    .collect();
                if !here.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        for (index, sb) in here {
                            if ui.button(&sb.label).clicked() {
                                clicked = Some(index);
                            }
                        }
                    });
                }
                for field in run {
                    render_field(ui, field, fields, state, visibility, Scope::Form);
                }
                for group in groups
                    .iter()
                    .filter(|g| g.section.as_deref() == Some(section.as_str()))
                {
                    render_group(ui, group, groups_state, visibility);
                }
            });
    }
    clicked
}

/// Whether `section` names a section some field in `fields` sits in — the
/// test for whether a button or group naming it is drawn there.
fn is_field_section(fields: &[FieldDef], section: Option<&str>) -> bool {
    section.is_some_and(|name| fields.iter().any(|f| f.section.as_deref() == Some(name)))
}

/// Renders one repeatable group as a list of add/remove-able instances, each
/// a mini form using the same [`render_field`] as the top-level fields.
fn render_group(
    ui: &mut egui::Ui,
    group: &GroupDef,
    groups_state: &mut GroupsState,
    visibility: &Visibility,
) {
    if !visibility.is_visible(group.visible_if.as_ref(), Scope::Form) {
        return;
    }
    // A selection-driven group has no manual add/remove — its blocks come and
    // go with the field named by `source`.
    let driven = group.source.is_some();

    ui.add_space(4.0);
    ui.separator();
    ui.strong(&group.label);

    let instances = groups_state.entry(group.key.clone()).or_default();
    let mut remove_index = None;
    for (i, instance) in instances.iter_mut().enumerate() {
        let scope = Scope::Instance(instance.id);
        ui.push_id(instance.id, |ui| {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    match &instance.source_item {
                        Some(item) => ui.label(format!("{} — {item}", group.label)),
                        None => ui.label(format!("{} {}", group.label, i + 1)),
                    };
                    if !driven && ui.small_button("Remove").clicked() {
                        remove_index = Some(i);
                    }
                });
                if !group.speed_buttons.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        for sb in &group.speed_buttons {
                            if !visibility.is_visible(sb.visible_if.as_ref(), scope) {
                                continue;
                            }
                            if ui.button(&sb.label).clicked() {
                                // Scoped to this block: fills in this block's
                                // own fields, leaving every other one alone.
                                apply_values(&group.fields, &sb.values, &mut instance.values);
                            }
                        }
                    });
                }
                for field in &group.fields {
                    render_field(ui, field, &group.fields, &mut instance.values, visibility, scope);
                }
            });
        });
    }
    if let Some(i) = remove_index {
        instances.remove(i);
    }

    if !driven && ui.button(format!("+ Add {}", group.label)).clicked() {
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
            ..Default::default()
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
            expr_eval: ExprEvaluator::default(),
        };
        (app, template)
    }

    #[test]
    fn form_is_valid_checks_required_fields_inside_group_instances() {
        let group = GroupDef {
            key: "procedures".to_string(),
            label: "Procedure Step".to_string(),
            fields: vec![text_field("tooth", true)],
            ..Default::default()
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
            app.form_is_valid(&template, &Visibility::default()),
            "no instances yet -> nothing to fail"
        );

        add_group_instance(&mut app.groups_state, &group);
        assert!(
            !app.form_is_valid(&template, &Visibility::default()),
            "a fresh instance has its required 'tooth' field blank"
        );

        let instances = app.groups_state.get_mut("procedures").unwrap();
        instances[0]
            .values
            .insert("tooth".to_string(), FieldValue::Text("14".to_string()));
        assert!(app.form_is_valid(&template, &Visibility::default()), "filled -> valid again");

        add_group_instance(&mut app.groups_state, &group);
        assert!(
            !app.form_is_valid(&template, &Visibility::default()),
            "one bad instance among several still blocks copy"
        );
    }

    #[test]
    fn form_is_valid_ignores_optional_group_fields_left_blank() {
        let group = GroupDef {
            key: "procedures".to_string(),
            label: "Procedure Step".to_string(),
            fields: vec![text_field("notes", false)],
            ..Default::default()
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
            app.form_is_valid(&template, &Visibility::default()),
            "optional field left blank shouldn't block copy"
        );
    }
}
