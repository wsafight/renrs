use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

impl fmt::Display for Severity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => formatter.write_str("error"),
            Self::Warning => formatter.write_str("warning"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub hint: Option<String>,
}

impl Diagnostic {
    #[must_use]
    pub fn new(
        file: impl Into<String>,
        line: usize,
        column: usize,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: Severity::Error,
            file: file.into(),
            line,
            column,
            message: message.into(),
            hint: None,
        }
    }

    #[must_use]
    pub fn warning(
        file: impl Into<String>,
        line: usize,
        column: usize,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: Severity::Warning,
            file: file.into(),
            line,
            column,
            message: message.into(),
            hint: None,
        }
    }

    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self.severity, Severity::Error)
    }

    #[must_use]
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}: {}: {}",
            self.file, self.line, self.column, self.severity, self.message
        )?;
        if let Some(hint) = &self.hint {
            write!(f, "\n  hint: {hint}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Diagnostic {}

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
