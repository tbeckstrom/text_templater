//! Headless template checker.
//!
//! Loads a templates directory (the live app-data one by default), reports any
//! load errors, and renders every template with default values so broken Tera
//! or a bad `use_partials`/`visible_if` shows up without opening the GUI.
//!
//!   cargo run --bin check_templates                 # the app's own templates
//!   cargo run --bin check_templates -- examples/templates
//!   cargo run --bin check_templates -- <dir> <template_id>   # print that note
//!
//! With a template id it prints the fully rendered note, which is the quickest
//! way to eyeball wording changes.

use std::path::PathBuf;

use text_templater::field::{build_form_state, build_groups_state, sync_source_group};
use text_templater::lint::lint;
use text_templater::template::loader::load_templates;
use text_templater::visibility::{recompute_fields, ExprEvaluator};
use text_templater::{formatting, render, storage};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir: PathBuf = match args.first() {
        Some(d) => PathBuf::from(d),
        None => storage::resolve_paths()
            .expect("could not resolve the app data directory")
            .templates_dir,
    };
    let only = args.get(1);

    println!("templates dir: {}", dir.display());
    let result = load_templates(&dir);

    if result.errors.is_empty() {
        println!("load errors:   none");
    } else {
        println!("load errors:   {}", result.errors.len());
        for (file, err) in &result.errors {
            println!("  ✗ {file}: {err}");
        }
    }

    println!(
        "partials:      {}",
        result
            .partials
            .iter()
            .map(|p| {
                let extras = p.fields.len() + p.groups.len() + p.speed_buttons.len();
                if extras == 0 {
                    p.name.clone()
                } else {
                    format!(
                        "{} ({} fields, {} groups, {} buttons)",
                        p.name,
                        p.fields.len(),
                        p.groups.len(),
                        p.speed_buttons.len()
                    )
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!();

    let mut failures = 0;
    for t in &result.templates {
        if only.is_some_and(|id| id != &t.id) {
            continue;
        }

        // Same startup path the app takes: defaults, then reconcile any
        // selection-driven groups.
        let mut state = build_form_state(&t.fields);
        let mut groups_state = build_groups_state(&t.groups);
        for g in &t.groups {
            sync_source_group(g, &state, &mut groups_state);
        }
        let mut ctx = render::build_context(&t.fields, &state, &t.groups, &groups_state);
        let mut evaluator = ExprEvaluator::default();
        recompute_fields(&t.fields, &mut state, &mut ctx, &mut evaluator);

        let rendered = render::render_body_with_context(&t.body, &result.partials, &ctx);

        match rendered {
            Ok(text) => {
                println!(
                    "  ✓ {:<22} {} fields, {} groups, {} buttons",
                    t.id,
                    t.fields.len(),
                    t.groups.len(),
                    t.speed_buttons.len()
                );
                if only.is_some() {
                    println!("\n--------------------------------\n");
                    println!("{}", formatting::to_plain_text(&formatting::parse(&text)));
                    println!("--------------------------------");
                }
            }
            Err(e) => {
                failures += 1;
                println!("  ✗ {:<22} render failed:\n{e}", t.id);
            }
        }
    }

    let warnings = lint(&result.templates, &result.partials);
    if warnings.is_empty() {
        println!("\nlint: no warnings");
    } else {
        println!("\nlint: {} warning(s)", warnings.len());
        for w in &warnings {
            println!("  ! {}: {}", w.subject, w.message);
        }
    }

    // Warnings alone don't fail the run — only genuine load/render errors do.
    if failures > 0 || !result.errors.is_empty() {
        std::process::exit(1);
    }
}
