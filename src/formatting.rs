//! A minimal formatting syntax for note bodies: `**bold**`, `*italic*`,
//! `__underline__`. Not full Markdown — just these three toggles, so a
//! template author can bold a section header without learning anything else.

#[derive(Debug, Clone, PartialEq)]
pub struct FormattedRun {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

enum Marker {
    Bold,
    Italic,
    Underline,
}

/// Whether a marker at `i` may *open* a span: it has to hug the text it
/// formats, so the character right after it must be real content — not a
/// space and not more of the same marker character.
///
/// This is what keeps fill-in blanks intact: `# __ to # __` and `The ___
/// root` are left as literal text rather than being read as underline
/// markers and silently deleted from the note.
fn can_open(chars: &[char], i: usize, len: usize, marker: char) -> bool {
    match chars.get(i + len) {
        Some(&next) => !next.is_whitespace() && next != marker,
        None => false,
    }
}

/// Whether a marker at `i` may *close* the span it belongs to: the character
/// right before it must likewise be real content.
fn can_close(chars: &[char], i: usize, marker: char) -> bool {
    match i.checked_sub(1).map(|prev| chars[prev]) {
        Some(prev) => !prev.is_whitespace() && prev != marker,
        None => false,
    }
}

/// Whether the run of `marker` characters starting at `j` is exactly `len`
/// long at its head — used to tell `**` (bold) apart from a lone `*` (italic).
fn is_marker_of_len(chars: &[char], j: usize, marker: char, len: usize) -> bool {
    if chars.get(j) != Some(&marker) {
        return false;
    }
    let doubled = chars.get(j + 1) == Some(&marker);
    if len == 2 { doubled } else { !doubled }
}

/// Whether a span opened at `i` is closed **on the same line**. A marker with
/// no partner is left as literal text rather than being silently dropped — an
/// unbalanced `**` in a template is a typo worth seeing, not worth quietly
/// deleting from a clinical note.
///
/// Confining spans to one line is what stops `# ***` from pairing with the
/// `**` of some unrelated heading further down the note, and means a stray
/// marker can never run away and format everything after it.
fn has_closing_partner(chars: &[char], i: usize, marker: char, len: usize) -> bool {
    let mut j = i + len;
    while j < chars.len() && chars[j] != '\n' {
        if is_marker_of_len(chars, j, marker, len) && can_close(chars, j, marker) {
            return true;
        }
        j += 1;
    }
    false
}

/// Splits `input` into runs of plain text tagged with which of bold/italic/
/// underline are active, based on `**`/`*`/`__` toggle markers. A marker left
/// unclosed by the end of the text just stops applying there — no error.
///
/// Markers only take effect when they hug the text they format (see
/// [`can_open`]), so underscores and asterisks used as blanks or as plain
/// punctuation survive into the note untouched.
pub fn parse(input: &str) -> Vec<FormattedRun> {
    let mut runs = Vec::new();
    let (mut bold, mut italic, mut underline) = (false, false, false);
    let mut buf = String::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        let doubled = chars.get(i + 1) == Some(&c);
        let marker = match c {
            '*' if doubled => Some((Marker::Bold, 2)),
            '_' if doubled => Some((Marker::Underline, 2)),
            '*' => Some((Marker::Italic, 1)),
            _ => None,
        };

        if let Some((marker, len)) = marker {
            let currently_open = match marker {
                Marker::Bold => bold,
                Marker::Italic => italic,
                Marker::Underline => underline,
            };
            let takes_effect = if currently_open {
                can_close(&chars, i, c)
            } else {
                can_open(&chars, i, len, c) && has_closing_partner(&chars, i, c, len)
            };
            if takes_effect {
                flush(&mut runs, &mut buf, bold, italic, underline);
                match marker {
                    Marker::Bold => bold = !bold,
                    Marker::Italic => italic = !italic,
                    Marker::Underline => underline = !underline,
                }
                i += len;
                continue;
            }
        }

        // Not a marker in this position — keep the character, and let the
        // next one be considered on its own (so a run like `___` is examined
        // character by character rather than swallowed).
        buf.push(c);
        i += 1;
    }
    flush(&mut runs, &mut buf, bold, italic, underline);
    runs
}

fn flush(runs: &mut Vec<FormattedRun>, buf: &mut String, bold: bool, italic: bool, underline: bool) {
    if !buf.is_empty() {
        runs.push(FormattedRun {
            text: std::mem::take(buf),
            bold,
            italic,
            underline,
        });
    }
}

/// The runs' text with all formatting markers stripped — what "Copy Plain
/// Text" copies, and what gets saved to history (so History/"Copy again"
/// stay clean regardless of formatting).
pub fn to_plain_text(runs: &[FormattedRun]) -> String {
    runs.iter().map(|r| r.text.as_str()).collect()
}

/// Renders `runs` as HTML (`<b>`/`<i>`/`<u>`, `<br>` for newlines), for the
/// rich half of a clipboard write. Wrapped in a `white-space: pre-wrap` div
/// so indentation and blank lines survive the paste.
pub fn to_html(runs: &[FormattedRun]) -> String {
    let mut body = String::new();
    for run in runs {
        let escaped = html_escape(&run.text).replace('\n', "<br>");
        let mut open = String::new();
        let mut close = String::new();
        if run.bold {
            open.push_str("<b>");
            close.insert_str(0, "</b>");
        }
        if run.italic {
            open.push_str("<i>");
            close.insert_str(0, "</i>");
        }
        if run.underline {
            open.push_str("<u>");
            close.insert_str(0, "</u>");
        }
        body.push_str(&open);
        body.push_str(&escaped);
        body.push_str(&close);
    }
    format!(r#"<div style="white-space: pre-wrap;">{body}</div>"#)
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_with_no_markers_is_unchanged() {
        let runs = parse("Hello, world");
        assert_eq!(to_plain_text(&runs), "Hello, world");
        assert_eq!(runs.len(), 1);
        assert!(!runs[0].bold && !runs[0].italic && !runs[0].underline);
    }

    #[test]
    fn bold_italic_underline_markers_toggle_correctly() {
        let runs = parse("**bold** *italic* __underline__ plain");
        assert_eq!(to_plain_text(&runs), "bold italic underline plain");

        let bold_run = runs.iter().find(|r| r.text == "bold").unwrap();
        assert!(bold_run.bold && !bold_run.italic && !bold_run.underline);

        let italic_run = runs.iter().find(|r| r.text == "italic").unwrap();
        assert!(italic_run.italic && !italic_run.bold && !italic_run.underline);

        let underline_run = runs.iter().find(|r| r.text == "underline").unwrap();
        assert!(underline_run.underline && !underline_run.bold && !underline_run.italic);

        let plain_run = runs.iter().find(|r| r.text == " plain").unwrap();
        assert!(!plain_run.bold && !plain_run.italic && !plain_run.underline);
    }

    #[test]
    fn markers_can_combine() {
        let runs = parse("**__both__**");
        let run = runs.iter().find(|r| r.text == "both").unwrap();
        assert!(run.bold && run.underline && !run.italic);
    }

    /// An unbalanced marker is kept verbatim rather than stripped: it's
    /// almost always a template typo, and showing it beats silently removing
    /// characters from a note.
    #[test]
    fn unclosed_marker_is_left_as_literal_text() {
        let runs = parse("**oops no closing marker");
        assert_eq!(to_plain_text(&runs), "**oops no closing marker");
        assert!(runs.iter().all(|r| !r.bold));
    }

    #[test]
    fn html_escapes_special_characters_and_preserves_newlines() {
        let runs = parse("**A & B**\nnext line");
        let html = to_html(&runs);
        assert!(html.contains("<b>A &amp; B</b>"));
        assert!(html.contains("<br>next line"));
    }

    /// Regression guard: fill-in blanks in clinical prose must reach the note
    /// intact. Before markers were required to hug their text, `# __ to # __`
    /// was read as an underline span and the blanks vanished entirely, which
    /// is worse than an obvious placeholder — the reader can't tell anything
    /// is missing.
    #[test]
    fn fill_in_blanks_are_left_alone() {
        for original in [
            "crestal/sulcular incision made from # __ to # __. Full thickness",
            "The ___ root was sectioned axially to facilitate removal",
            "# ***",
            "Alveoloplasty of ______",
            "2 * 3 * 4",
            "snake_case_identifier stays whole",
        ] {
            assert_eq!(
                to_plain_text(&parse(original)),
                original,
                "expected {original:?} to survive unchanged"
            );
        }
    }

    #[test]
    fn markers_still_apply_when_they_hug_their_text() {
        let runs = parse("Tooth #14 *distoangular* impaction");
        assert_eq!(to_plain_text(&runs), "Tooth #14 distoangular impaction");
        assert!(runs.iter().any(|r| r.text == "distoangular" && r.italic));
    }

    #[test]
    fn a_span_does_not_run_across_lines() {
        // The `**` on the first line has no partner on that line, so it stays
        // literal instead of bolding everything down to the next heading.
        let runs = parse("# ***\nplain line\n**Heading**");
        let plain = to_plain_text(&runs);
        assert!(plain.starts_with("# ***"), "got {plain:?}");
        assert!(runs.iter().any(|r| r.text == "Heading" && r.bold));
        assert!(runs.iter().any(|r| r.text.contains("plain line") && !r.bold));
    }

    #[test]
    fn a_marker_padded_with_spaces_is_literal() {
        // The opening marker doesn't hug its text, so nothing is formatted.
        let runs = parse("a ** b ** c");
        assert_eq!(to_plain_text(&runs), "a ** b ** c");
        assert!(runs.iter().all(|r| !r.bold));
    }

    #[test]
    fn html_escapes_angle_brackets_in_plain_runs() {
        let runs = parse("<script>");
        let html = to_html(&runs);
        assert!(html.contains("&lt;script&gt;"));
        assert!(!html.contains("<script>"));
    }
}
