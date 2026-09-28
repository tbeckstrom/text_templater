//! For the browser build, compiles `my_templates/` into the binary (there is
//! no filesystem to load templates from at runtime). Generates
//! `$OUT_DIR/embedded_templates.rs`: a list of (path relative to
//! `my_templates/`, file contents) pairs that `storage` serves to the loader.
//! Native builds get an empty list and keep reading the templates directory.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::{env, fs};

const TEMPLATES_DIR: &str = "my_templates";

fn main() {
    println!("cargo:rerun-if-changed={TEMPLATES_DIR}");

    let root = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join(TEMPLATES_DIR);
    let mut files = Vec::new();
    if env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        collect(&root, &mut files);
        files.sort();
    }

    let mut out = String::from("pub const EMBEDDED_TEMPLATES: &[(&str, &str)] = &[\n");
    for path in &files {
        let relative = path.strip_prefix(&root).unwrap();
        let key = relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        writeln!(out, "    ({key:?}, include_str!({:?})),", path.display().to_string()).unwrap();
    }
    out.push_str("];\n");

    let dest = Path::new(&env::var("OUT_DIR").unwrap()).join("embedded_templates.rs");
    fs::write(dest, out).unwrap();
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for path in entries.filter_map(|e| e.ok()).map(|e| e.path()) {
        if path.is_dir() {
            collect(&path, out);
        } else if matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("toml" | "tera")
        ) {
            out.push(path);
        }
    }
}
