use std::fs;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;

pub struct Paths {
    pub templates_dir: PathBuf,
    pub history_file: PathBuf,
    pub drafts_file: PathBuf,
}

const SHARED_FIELDS_TOML: &str = include_str!("../examples/templates/shared_fields.toml");
const FOLLOW_UP_VISIT_TOML: &str = include_str!("../examples/templates/follow_up_visit.toml");
const INITIAL_CONSULT_TOML: &str = include_str!("../examples/templates/initial_consult.toml");
const ORAL_SURGERY_TOML: &str = include_str!("../examples/templates/oral_surgery.toml");
const HEADER_PARTIAL_TERA: &str = include_str!("../examples/templates/partials/header.tera");

pub fn resolve_paths() -> anyhow::Result<Paths> {
    let proj = ProjectDirs::from("", "", "note-templater")
        .ok_or_else(|| anyhow::anyhow!("could not determine app data directory"))?;
    let data_dir = proj.data_dir();
    let templates_dir = data_dir.join("templates");
    fs::create_dir_all(&templates_dir)?;

    seed_examples_if_empty(&templates_dir)?;

    Ok(Paths {
        templates_dir,
        history_file: data_dir.join("history.jsonl"),
        drafts_file: data_dir.join("drafts.json"),
    })
}

fn seed_examples_if_empty(templates_dir: &Path) -> anyhow::Result<()> {
    let is_empty = fs::read_dir(templates_dir)?.next().is_none();
    if !is_empty {
        return Ok(());
    }
    fs::write(templates_dir.join("shared_fields.toml"), SHARED_FIELDS_TOML)?;
    fs::write(
        templates_dir.join("follow_up_visit.toml"),
        FOLLOW_UP_VISIT_TOML,
    )?;
    fs::write(
        templates_dir.join("initial_consult.toml"),
        INITIAL_CONSULT_TOML,
    )?;
    fs::write(templates_dir.join("oral_surgery.toml"), ORAL_SURGERY_TOML)?;

    let partials_dir = templates_dir.join("partials");
    fs::create_dir_all(&partials_dir)?;
    fs::write(partials_dir.join("header.tera"), HEADER_PARTIAL_TERA)?;

    Ok(())
}
