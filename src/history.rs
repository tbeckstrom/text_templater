use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub timestamp: String,
    pub template_id: String,
    pub template_name: String,
    pub rendered_text: String,
    /// The form field values at copy time (see `field::form_state_to_json`), so
    /// clicking this entry can refill the form. `#[serde(default)]` so history
    /// written before this field existed still loads (just without restore data).
    #[serde(default)]
    pub field_values: serde_json::Value,
}

pub fn load_history(path: &Path) -> Vec<HistoryEntry> {
    let Ok(text) = crate::storage::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

pub fn append_history(path: &Path, entry: &HistoryEntry) -> std::io::Result<()> {
    let json = serde_json::to_string(entry).expect("HistoryEntry always serializes");
    crate::storage::append_line(path, &json)
}

pub fn now_timestamp() -> String {
    jiff::Zoned::now()
        .strftime("%Y-%m-%d %H:%M:%S")
        .to_string()
}
