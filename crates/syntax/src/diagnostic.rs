pub use velin_syntax::{Diagnostic, Severity};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_errors_and_warnings_with_optional_hints() {
        let error = Diagnostic::new("story.rns", 4, 8, "unknown label");
        assert!(error.is_error());
        assert_eq!(error.to_string(), "story.rns:4:8: error: unknown label");

        let warning =
            Diagnostic::warning("story.rns", 2, 1, "unused").with_hint("delete the label");
        assert!(!warning.is_error());
        assert_eq!(
            warning.to_string(),
            "story.rns:2:1: warning: unused\n  hint: delete the label"
        );
    }
}
