use crate::diagnostic::Diagnostic;
use crate::parser::parse_document;

/// Applies conservative formatting without changing indentation or comments.
#[must_use]
pub fn format_source(source: &str) -> String {
    let mut output = String::new();
    let mut blank_lines = 0_usize;
    for raw in source.lines() {
        let line = raw.trim_end_matches([' ', '\t']);
        if line.is_empty() {
            blank_lines += 1;
            if blank_lines > 2 || output.is_empty() {
                continue;
            }
        } else {
            blank_lines = 0;
        }
        output.push_str(line);
        output.push('\n');
    }
    while output.ends_with("\n\n") {
        output.pop();
    }
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }
    output
}

#[must_use]
pub fn document_diagnostics(source: &str, source_name: &str) -> Vec<Diagnostic> {
    parse_document(source, source_name)
        .err()
        .unwrap_or_default()
}
