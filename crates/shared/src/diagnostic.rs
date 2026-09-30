//! Diagnostic formatting helpers.

/// Joins diagnostics into one newline-separated message.
#[must_use]
pub fn join_diagnostics<T: std::fmt::Display>(diagnostics: &[T]) -> String {
    diagnostics
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}
