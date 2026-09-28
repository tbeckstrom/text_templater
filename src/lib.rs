//! Note Templater — TOML-defined note templates rendered through Tera.
//!
//! Exposed as a library so the GUI binary and the headless `check_templates`
//! tool share exactly one copy of the loading and rendering logic.

pub mod app;
pub mod drafts;
pub mod field;
pub mod formatting;
pub mod history;
pub mod lint;
pub mod render;
pub mod storage;
pub mod template;
pub mod typography;
pub mod visibility;
