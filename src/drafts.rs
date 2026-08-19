use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::history::now_timestamp;

/// How many templates' in-progress drafts to keep. Each template has at most
/// one draft (saving a newer one replaces the old one), so this is really a
/// cap on "how many different templates you can have unsaved work in at once".
pub const MAX_DRAFTS: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftEntry {
    pub template_id: String,
    pub template_name: String,
    pub updated_at: String,
    pub field_values: serde_json::Value,
}

pub fn load_drafts(path: &Path) -> Vec<DraftEntry> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

pub fn save_drafts(path: &Path, drafts: &[DraftEntry]) -> std::io::Result<()> {
    let json = serde_json::to_string(drafts).expect("Vec<DraftEntry> always serializes");
    std::fs::write(path, json)
}

/// Replaces any existing draft for `template_id` with `field_values`, moving it
/// to the front (most-recently-touched first), or removes it entirely if
/// `field_values` has no meaningful content (`has_progress` is false). Then
/// truncates to [`MAX_DRAFTS`] and persists.
pub fn upsert(
    path: &Path,
    drafts: &mut Vec<DraftEntry>,
    template_id: &str,
    template_name: &str,
    field_values: serde_json::Value,
    has_progress: bool,
) {
    drafts.retain(|d| d.template_id != template_id);
    if has_progress {
        drafts.insert(
            0,
            DraftEntry {
                template_id: template_id.to_string(),
                template_name: template_name.to_string(),
                updated_at: now_timestamp(),
                field_values,
            },
        );
        drafts.truncate(MAX_DRAFTS);
    }
    let _ = save_drafts(path, drafts);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "note_templater_drafts_test_{}_{}",
            std::process::id(),
            name
        ))
    }

    #[test]
    fn upsert_adds_then_updates_in_place_at_front() {
        let path = scratch_path("upsert_update");
        let mut drafts = Vec::new();

        upsert(&path, &mut drafts, "a", "Template A", serde_json::json!({"x": 1}), true);
        upsert(&path, &mut drafts, "b", "Template B", serde_json::json!({"x": 2}), true);
        assert_eq!(drafts.len(), 2);
        assert_eq!(drafts[0].template_id, "b"); // most recently touched first

        // Re-saving "a" should update it in place and move it to the front,
        // not create a second entry.
        upsert(&path, &mut drafts, "a", "Template A", serde_json::json!({"x": 3}), true);
        assert_eq!(drafts.len(), 2);
        assert_eq!(drafts[0].template_id, "a");
        assert_eq!(drafts[0].field_values, serde_json::json!({"x": 3}));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn upsert_with_no_progress_removes_existing_draft() {
        let path = scratch_path("upsert_remove");
        let mut drafts = Vec::new();
        upsert(&path, &mut drafts, "a", "Template A", serde_json::json!({"x": 1}), true);
        assert_eq!(drafts.len(), 1);

        // Simulates clearing a draft after copy: has_progress=false drops it.
        upsert(&path, &mut drafts, "a", "Template A", serde_json::Value::Null, false);
        assert!(drafts.is_empty());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn upsert_truncates_to_max_drafts() {
        let path = scratch_path("upsert_truncate");
        let mut drafts = Vec::new();
        for i in 0..(MAX_DRAFTS + 3) {
            upsert(
                &path,
                &mut drafts,
                &format!("template_{i}"),
                "Some Template",
                serde_json::json!({}),
                true,
            );
        }
        assert_eq!(drafts.len(), MAX_DRAFTS);
        // Most recent (highest i) should have survived; oldest were evicted.
        assert_eq!(drafts[0].template_id, format!("template_{}", MAX_DRAFTS + 2));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn load_drafts_round_trips_through_save() {
        let path = scratch_path("round_trip");
        let mut drafts = Vec::new();
        upsert(&path, &mut drafts, "a", "Template A", serde_json::json!({"x": 1}), true);

        let loaded = load_drafts(&path);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].template_id, "a");

        std::fs::remove_file(&path).ok();
    }
}
