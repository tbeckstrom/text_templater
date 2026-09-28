//! Headless template checker.
//!
//! Loads a templates directory (the live app-data one by default), reports any
//! load errors, and renders every template with default values so broken Tera
//! or a bad `use_partials`/`visible_if` shows up without opening the GUI.
//!
//!   cargo run --bin check_templates                 # the app's own templates
//!   cargo run --bin check_templates -- examples/templates
//!   cargo run --bin check_templates -- <dir> <template_id>   # print that note
//!   cargo run --bin check_templates -- <dir> --fix  # replace AI-style characters
//!
//! With a template id it prints the fully rendered note, which is the quickest
//! way to eyeball wording changes.
//!
//! It also reports em dashes, curly quotes and other AI-style characters in
//! the template files (see `typography`). `--fix` rewrites those files with
//! plain keyboard equivalents before checking them.

use std::path::{Path, PathBuf};

use text_templater::field::{build_form_state, build_groups_state, sync_source_group};
use text_templater::lint::lint;
use text_templater::template::loader::load_templates;
use text_templater::visibility::{recompute_fields, ExprEvaluator};
use text_templater::{formatting, render, storage, typography};

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let apply_fix = args.iter().any(|a| a == "--fix");
    args.retain(|a| a != "--fix");
    let dir: PathBuf = match args.first() {
        Some(d) => PathBuf::from(d),
        None => storage::resolve_paths()
            .expect("could not resolve the app data directory")
            .templates_dir,
    };
    let only = args.get(1);

    println!("templates dir: {}", dir.display());
    // Fix first, so everything below checks the rewritten files.
    let typography_report = check_typography(&dir, apply_fix);
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

    println!("{typography_report}");

    // Warnings alone don't fail the run — only genuine load/render errors do.
    if failures > 0 || !result.errors.is_empty() {
        std::process::exit(1);
    }
}

/// Scans (and with `fix`, rewrites) every `.toml` file under `dir` for
/// AI-style characters, returning the report to print.
fn check_typography(dir: &Path, fix: bool) -> String {
    let mut files = Vec::new();
    collect_toml_files(dir, &mut files);
    files.sort();

    let mut lines = Vec::new();
    let mut affected = 0;
    for file in &files {
        let Ok(source) = std::fs::read_to_string(file) else { continue };
        // A file that doesn't parse is already reported as a load error.
        let Ok(findings) = typography::scan(&source) else { continue };
        if findings.is_empty() {
            continue;
        }
        affected += 1;
        let name = file.strip_prefix(dir).unwrap_or(file).display();
        if fix {
            match typography::fix(&source).map(|fixed| std::fs::write(file, fixed)) {
                Ok(Ok(())) => lines.push(format!("  ✓ fixed {name} ({} value(s))", findings.len())),
                Ok(Err(e)) => lines.push(format!("  ✗ {name}: could not write: {e}")),
                Err(e) => lines.push(format!("  ✗ {name}: {e}")),
            }
        } else {
            for f in &findings {
                lines.push(format!("  ! {name}: {}: {}", f.path, f.characters.join(", ")));
            }
        }
    }

    if affected == 0 {
        "typography: no AI-style characters".to_string()
    } else if fix {
        format!("typography: fixed {affected} file(s)\n{}", lines.join("\n"))
    } else {
        format!(
            "typography: {affected} file(s) with AI-style characters (run with --fix to replace)\n{}",
            lines.join("\n")
        )
    }
}

fn collect_toml_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_toml_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "toml") {
            out.push(path);
        }
    }
}
