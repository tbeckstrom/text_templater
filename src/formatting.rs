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

/// Splits `input` into runs of plain text tagged with which of bold/italic/
/// underline are active, based on `**`/`*`/`__` toggle markers. A marker left
/// unclosed by the end of the text just stops applying there — no error.
pub fn parse(input: &str) -> Vec<FormattedRun> {
    let mut runs = Vec::new();
    let (mut bold, mut italic, mut underline) = (false, false, false);
    let mut buf = String::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let two_ahead = chars.get(i + 1);
        if chars[i] == '*' && two_ahead == Some(&'*') {
            flush(&mut runs, &mut buf, bold, italic, underline);
            bold = !bold;
            i += 2;
        } else if chars[i] == '_' && two_ahead == Some(&'_') {
            flush(&mut runs, &mut buf, bold, italic, underline);
            underline = !underline;
            i += 2;
        } else if chars[i] == '*' {
            flush(&mut runs, &mut buf, bold, italic, underline);
            italic = !italic;
            i += 1;
        } else {
            buf.push(chars[i]);
            i += 1;
        }
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

    #[test]
    fn unclosed_marker_does_not_panic_and_still_produces_text() {
        let runs = parse("**oops no closing marker");
        assert_eq!(to_plain_text(&runs), "oops no closing marker");
    }

    #[test]
    fn html_escapes_special_characters_and_preserves_newlines() {
        let runs = parse("**A & B**\nnext line");
        let html = to_html(&runs);
        assert!(html.contains("<b>A &amp; B</b>"));
        assert!(html.contains("<br>next line"));
    }

    #[test]
    fn html_escapes_angle_brackets_in_plain_runs() {
        let runs = parse("<script>");
        let html = to_html(&runs);
        assert!(html.contains("&lt;script&gt;"));
        assert!(!html.contains("<script>"));
    }
}
