//! Typographic characters that AI-drafted text tends to carry (em dashes,
//! curly quotes, ellipses, invisible spaces, ≥ and →), and the plain keyboard
//! characters that replace them.
//!
//! This works on a template file's TOML source rather than on loaded
//! templates, because the fix rewrites the file: only string values change,
//! and each keeps its original quoting style, so comments and layout stay
//! exactly as written.

use toml_edit::{DocumentMut, Formatted, Item, Value};

/// Each character, its keyboard equivalent, and a name for the report.
const REPLACEMENTS: &[(char, &str, &str)] = &[
    ('\u{2014}', "-", "em dash"),
    ('\u{2013}', "-", "en dash"),
    ('\u{2012}', "-", "figure dash"),
    ('\u{2015}', "-", "horizontal bar"),
    ('\u{2010}', "-", "Unicode hyphen"),
    ('\u{2011}', "-", "non-breaking hyphen"),
    ('\u{2212}', "-", "minus sign"),
    ('\u{2018}', "'", "left single quote"),
    ('\u{2019}', "'", "right single quote"),
    ('\u{201A}', "'", "low single quote"),
    ('\u{201B}', "'", "reversed single quote"),
    ('\u{2032}', "'", "prime"),
    ('\u{201C}', "\"", "left double quote"),
    ('\u{201D}', "\"", "right double quote"),
    ('\u{201E}', "\"", "low double quote"),
    ('\u{201F}', "\"", "reversed double quote"),
    ('\u{2033}', "\"", "double prime"),
    ('\u{2026}', "...", "ellipsis"),
    ('\u{00A0}', " ", "non-breaking space"),
    ('\u{2002}', " ", "en space"),
    ('\u{2003}', " ", "em space"),
    ('\u{2004}', " ", "three-per-em space"),
    ('\u{2005}', " ", "four-per-em space"),
    ('\u{2006}', " ", "six-per-em space"),
    ('\u{2007}', " ", "figure space"),
    ('\u{2008}', " ", "punctuation space"),
    ('\u{2009}', " ", "thin space"),
    ('\u{200A}', " ", "hair space"),
    ('\u{202F}', " ", "narrow no-break space"),
    ('\u{200B}', "", "zero-width space"),
    ('\u{200C}', "", "zero-width non-joiner"),
    ('\u{200D}', "", "zero-width joiner"),
    ('\u{2060}', "", "word joiner"),
    ('\u{FEFF}', "", "zero-width no-break space"),
    ('\u{2022}', "-", "bullet"),
    ('\u{2023}', "-", "triangular bullet"),
    ('\u{2043}', "-", "hyphen bullet"),
    ('\u{25E6}', "-", "white bullet"),
    ('\u{2265}', ">=", "greater-than or equal"),
    ('\u{2264}', "<=", "less-than or equal"),
    ('\u{2260}', "!=", "not equal"),
    ('\u{00B1}', "+/-", "plus-minus"),
    ('\u{00D7}', "x", "multiplication sign"),
    ('\u{2192}', "->", "right arrow"),
    ('\u{2190}', "<-", "left arrow"),
    ('\u{2194}', "<->", "left-right arrow"),
    ('\u{21D2}', "=>", "double right arrow"),
];

fn lookup(c: char) -> Option<(&'static str, &'static str)> {
    REPLACEMENTS
        .iter()
        .find(|(from, _, _)| *from == c)
        .map(|(_, to, name)| (*to, *name))
}

/// `text` with every listed character swapped for its keyboard equivalent.
pub fn normalize(text: &str) -> String {
    convert(text, false)
}

/// The replacement pass shared by [`normalize`] and the raw-source rewrite.
/// An em dash set tight between two words ("techniques—including") becomes
/// a spaced " - ", so the words don't run together as a hyphenated compound.
/// With `escape_quotes`, a straight double quote comes out as `\"`, for use
/// inside a TOML basic string.
fn convert(text: &str, escape_quotes: bool) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    for (i, &c) in chars.iter().enumerate() {
        let tight = |j: Option<usize>| j.and_then(|j| chars.get(j)).is_some_and(|n| n.is_alphanumeric());
        match lookup(c) {
            Some(("\"", _)) if escape_quotes => out.push_str("\\\""),
            Some(_) if c == '\u{2014}' && tight(i.checked_sub(1)) && tight(Some(i + 1)) => {
                out.push_str(" - ")
            }
            Some((to, _)) => out.push_str(to),
            None => out.push(c),
        }
    }
    out
}

/// One string value in a file that contains replaceable characters.
#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    /// Where the value sits, e.g. `fields[imp_exam_mio_mm].options[0]`.
    pub path: String,
    /// Each distinct character found, described as `'–' en dash -> '-'`.
    pub characters: Vec<String>,
}

/// Every string value in `source` that `fix` would change.
pub fn scan(source: &str) -> Result<Vec<Finding>, String> {
    let mut doc: DocumentMut = source.parse().map_err(|e| format!("{e}"))?;
    let mut findings = Vec::new();
    visit_table(doc.as_table_mut(), "", &mut |path, s| {
        let mut characters: Vec<String> = Vec::new();
        for c in s.value().chars() {
            if let Some((to, name)) = lookup(c) {
                let described = format!("'{c}' {name} -> '{to}'");
                if !characters.contains(&described) {
                    characters.push(described);
                }
            }
        }
        if !characters.is_empty() {
            findings.push(Finding { path: path.to_string(), characters });
        }
    });
    Ok(findings)
}

/// `source` with the listed characters replaced in every string value.
pub fn fix(source: &str) -> Result<String, String> {
    let mut doc: DocumentMut = source.parse().map_err(|e| format!("{e}"))?;
    visit_table(doc.as_table_mut(), "", &mut |_, s| {
        if normalize(s.value()) != *s.value() {
            *s = rewritten(s);
        }
    });
    Ok(doc.to_string())
}

/// The normalized string, written in the same quoting style as the original.
///
/// The replacement is made in the raw source text so a `"""` body stays a
/// `"""` body. A straight double quote needs escaping inside a basic string,
/// and a straight apostrophe cannot go inside a single-line literal string
/// at all. If the edited source doesn't parse back to exactly the normalized
/// value, the string is re-encoded from scratch instead.
fn rewritten(original: &Formatted<String>) -> Formatted<String> {
    let target = normalize(original.value());
    let raw = original.as_repr().and_then(|r| r.as_raw().as_str());
    let mut result = raw
        .and_then(|raw| {
            let edited = convert(raw, raw.starts_with('"'));
            match edited.parse::<Value>() {
                Ok(Value::String(parsed)) if *parsed.value() == target => Some(parsed),
                _ => None,
            }
        })
        .unwrap_or_else(|| Formatted::new(target));
    *result.decor_mut() = original.decor().clone();
    result
}

fn visit_table(
    table: &mut toml_edit::Table,
    path: &str,
    f: &mut dyn FnMut(&str, &mut Formatted<String>),
) {
    for (key, item) in table.iter_mut() {
        visit_item(item, &join(path, key.get()), f);
    }
}

fn visit_item(item: &mut Item, path: &str, f: &mut dyn FnMut(&str, &mut Formatted<String>)) {
    match item {
        Item::Value(v) => visit_value(v, path, f),
        Item::Table(t) => visit_table(t, path, f),
        Item::ArrayOfTables(tables) => {
            for (i, t) in tables.iter_mut().enumerate() {
                let label = entry_label(t.get("key").and_then(Item::as_str), i);
                visit_table(t, &format!("{path}[{label}]"), f);
            }
        }
        Item::None => {}
    }
}

fn visit_value(value: &mut Value, path: &str, f: &mut dyn FnMut(&str, &mut Formatted<String>)) {
    match value {
        Value::String(s) => f(path, s),
        Value::Array(items) => {
            for (i, v) in items.iter_mut().enumerate() {
                visit_value(v, &format!("{path}[{i}]"), f);
            }
        }
        Value::InlineTable(t) => {
            for (key, v) in t.iter_mut() {
                visit_value(v, &join(path, key.get()), f);
            }
        }
        _ => {}
    }
}

/// A `[[fields]]` entry is easier to find by its key than by its position.
fn entry_label(key: Option<&str>, index: usize) -> String {
    key.map_or_else(|| index.to_string(), str::to_string)
}

fn join(path: &str, key: &str) -> String {
    if path.is_empty() { key.to_string() } else { format!("{path}.{key}") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_replaces_each_kind_of_character() {
        assert_eq!(normalize("HPI – consult — note"), "HPI - consult - note");
        assert_eq!(normalize("the patient’s “goal”"), "the patient's \"goal\"");
        assert_eq!(normalize("wait…"), "wait...");
        assert_eq!(normalize("10\u{00A0}mm\u{200B}"), "10 mm");
        assert_eq!(normalize("• item"), "- item");
        assert_eq!(normalize("≥2 mm, ≤4 mm, 7 × 10, ±graft, ext → delayed"), ">=2 mm, <=4 mm, 7 x 10, +/-graft, ext -> delayed");
    }

    #[test]
    fn an_em_dash_between_two_words_becomes_a_spaced_hyphen() {
        assert_eq!(normalize("techniques—including blocks"), "techniques - including blocks");
        assert_eq!(normalize("a — b"), "a - b");
        assert_eq!(normalize("3–4 months"), "3-4 months");
        let fixed = fix("text = \"techniques—including\"\n").unwrap();
        assert_eq!(fixed, "text = \"techniques - including\"\n");
    }

    #[test]
    fn normalize_leaves_plain_text_and_other_symbols_alone() {
        assert_eq!(normalize("A1c 6.8% at 45° - ok"), "A1c 6.8% at 45° - ok");
    }

    #[test]
    fn fix_changes_only_the_affected_strings_and_keeps_comments_and_layout() {
        let source = "# a comment — left alone\nname = \"Snippet: HPI – consult\"\nbody = \"\"\"\nLine one — here.\n\"\"\"\n\n[[fields]]\nkey = \"a\"\nlabel = \"Plain\"\n";
        let fixed = fix(source).unwrap();
        assert_eq!(
            fixed,
            "# a comment — left alone\nname = \"Snippet: HPI - consult\"\nbody = \"\"\"\nLine one - here.\n\"\"\"\n\n[[fields]]\nkey = \"a\"\nlabel = \"Plain\"\n"
        );
    }

    #[test]
    fn a_curly_double_quote_is_escaped_inside_a_basic_string() {
        let fixed = fix("text = \"CC: “pain”\"\n").unwrap();
        assert_eq!(fixed, "text = \"CC: \\\"pain\\\"\"\n");
        let doc: DocumentMut = fixed.parse().unwrap();
        assert_eq!(doc["text"].as_str(), Some("CC: \"pain\""));
    }

    #[test]
    fn a_curly_apostrophe_in_a_literal_string_is_re_encoded_validly() {
        let fixed = fix("text = 'patient’s goal'\n").unwrap();
        let doc: DocumentMut = fixed.parse().unwrap();
        assert_eq!(doc["text"].as_str(), Some("patient's goal"));
    }

    #[test]
    fn inline_option_tables_and_preset_values_are_fixed_too() {
        let source = "options = [{ label = \"3–4 months\", text = \"≥2 mm\" }]\nvalues = { k = \"3–4 months\" }\n";
        let doc: DocumentMut = fix(source).unwrap().parse().unwrap();
        assert_eq!(doc["options"][0]["label"].as_str(), Some("3-4 months"));
        assert_eq!(doc["options"][0]["text"].as_str(), Some(">=2 mm"));
        assert_eq!(doc["values"]["k"].as_str(), Some("3-4 months"));
    }

    #[test]
    fn a_clean_file_is_returned_unchanged() {
        let source = "name = \"Plain\"\n# note — comment\n[[fields]]\nkey = \"a\"\n";
        assert_eq!(fix(source).unwrap(), source);
        assert!(scan(source).unwrap().is_empty());
    }

    #[test]
    fn scan_names_the_field_by_key_and_lists_each_character_once() {
        let source = "[[fields]]\nkey = \"imp_ap_graft_time\"\noptions = [\"4 months\", \"4–6 months – ok\"]\n";
        let findings = scan(source).unwrap();
        assert_eq!(
            findings,
            vec![Finding {
                path: "fields[imp_ap_graft_time].options[1]".to_string(),
                characters: vec!["'–' en dash -> '-'".to_string()],
            }]
        );
    }
}
