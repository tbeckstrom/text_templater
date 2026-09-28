//! Where the app keeps its data, and the handful of file operations the rest
//! of the app goes through. On the desktop these are the real filesystem; in
//! the browser (wasm32) templates come from `my_templates/` compiled into the
//! binary (see `build.rs`) and history/drafts live in `localStorage`.

use std::io;
use std::path::{Path, PathBuf};

pub use backend::{append_line, exists, read_dir, read_to_string, resolve_paths, write};

pub struct Paths {
    pub templates_dir: PathBuf,
    pub history_file: PathBuf,
    pub drafts_file: PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use std::fs;
    use std::io::Write as _;

    use directories::ProjectDirs;

    use super::*;

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

    pub fn read_to_string(path: &Path) -> io::Result<String> {
        fs::read_to_string(path)
    }

    pub fn write(path: &Path, contents: &str) -> io::Result<()> {
        fs::write(path, contents)
    }

    pub fn append_line(path: &Path, line: &str) -> io::Result<()> {
        let mut file = fs::OpenOptions::new().create(true).append(true).open(path)?;
        writeln!(file, "{line}")
    }

    pub fn read_dir(dir: &Path) -> io::Result<Vec<PathBuf>> {
        Ok(fs::read_dir(dir)?.filter_map(|e| e.ok()).map(|e| e.path()).collect())
    }

    pub fn exists(path: &Path) -> bool {
        path.exists()
    }
}

#[cfg(target_arch = "wasm32")]
mod backend {
    use super::*;

    include!(concat!(env!("OUT_DIR"), "/embedded_templates.rs"));

    /// The virtual directory the embedded templates appear under.
    const TEMPLATES_DIR: &str = "templates";
    /// Prefix for every `localStorage` key, so the app's entries are easy to
    /// find (and clear) in the browser's storage inspector.
    const KEY_PREFIX: &str = "note-templater/";

    pub fn resolve_paths() -> anyhow::Result<Paths> {
        Ok(Paths {
            templates_dir: PathBuf::from(TEMPLATES_DIR),
            history_file: PathBuf::from("history.jsonl"),
            drafts_file: PathBuf::from("drafts.json"),
        })
    }

    /// `path`'s location inside the embedded templates, if it's under them.
    fn embedded_key(path: &Path) -> Option<String> {
        let relative = path.strip_prefix(TEMPLATES_DIR).ok()?;
        Some(
            relative
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/"),
        )
    }

    fn local_storage() -> io::Result<web_sys::Storage> {
        web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .ok_or_else(|| io::Error::other("browser storage is unavailable"))
    }

    fn storage_key(path: &Path) -> String {
        format!("{KEY_PREFIX}{}", path.display())
    }

    pub fn read_to_string(path: &Path) -> io::Result<String> {
        let found = match embedded_key(path) {
            Some(key) => EMBEDDED_TEMPLATES
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, text)| text.to_string()),
            None => local_storage()?
                .get_item(&storage_key(path))
                .map_err(|_| io::Error::other("could not read browser storage"))?,
        };
        found.ok_or_else(|| io::ErrorKind::NotFound.into())
    }

    pub fn write(path: &Path, contents: &str) -> io::Result<()> {
        if embedded_key(path).is_some() {
            return Err(io::Error::other("templates are read-only in the browser"));
        }
        local_storage()?
            .set_item(&storage_key(path), contents)
            .map_err(|_| io::Error::other("browser storage is full or blocked"))
    }

    pub fn append_line(path: &Path, line: &str) -> io::Result<()> {
        let mut text = read_to_string(path).unwrap_or_default();
        text.push_str(line);
        text.push('\n');
        write(path, &text)
    }

    /// The immediate children of `dir` among the embedded templates. A
    /// subdirectory shows up once, as a path without an extension.
    pub fn read_dir(dir: &Path) -> io::Result<Vec<PathBuf>> {
        let prefix = embedded_key(dir).ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
        let prefix = if prefix.is_empty() { prefix } else { format!("{prefix}/") };
        let mut children: Vec<PathBuf> = Vec::new();
        for (key, _) in EMBEDDED_TEMPLATES {
            let Some(rest) = key.strip_prefix(prefix.as_str()) else { continue };
            let child = dir.join(rest.split('/').next().unwrap_or(rest));
            if !children.contains(&child) {
                children.push(child);
            }
        }
        Ok(children)
    }

    pub fn exists(path: &Path) -> bool {
        match embedded_key(path) {
            Some(key) if key.is_empty() => true,
            Some(key) => EMBEDDED_TEMPLATES
                .iter()
                .any(|(k, _)| *k == key || k.starts_with(&format!("{key}/"))),
            None => read_to_string(path).is_ok(),
        }
    }
}
