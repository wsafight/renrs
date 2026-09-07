use std::io::{self, Write};

use serde::Serialize;

use crate::Diagnostic;

pub const PROTOCOL_VERSION: u32 = 1;
pub const EXIT_FAILURE: u8 = 1;
pub const EXIT_USAGE: u8 = 2;

pub mod code {
    pub const ACCEPTANCE_FAILED: &str = "acceptance_failed";
    pub const EXPLORATION_INCOMPLETE: &str = "exploration_incomplete";
    pub const GIT_BASELINE_FAILED: &str = "git_baseline_failed";
    pub const IMPACT_FAILED: &str = "impact_failed";
    pub const INVALID_ARGUMENTS: &str = "invalid_arguments";
    pub const PROJECT_INVALID: &str = "project_invalid";
    pub const ROUTE_FAILED: &str = "route_failed";
    pub const RUNTIME_FAILED: &str = "runtime_failed";
    pub const VALIDATION_FAILED: &str = "validation_failed";
}

#[derive(Debug, Clone, Serialize)]
pub struct MachineError {
    pub code: String,
    pub message: String,
}

impl MachineError {
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MachineDiagnostic {
    pub code: String,
    pub severity: crate::diagnostic::Severity,
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub hint: Option<String>,
}

impl From<Diagnostic> for MachineDiagnostic {
    fn from(diagnostic: Diagnostic) -> Self {
        let code = if diagnostic.is_error() {
            "validation_error"
        } else {
            "analysis_warning"
        };
        Self {
            code: code.to_owned(),
            severity: diagnostic.severity,
            file: diagnostic.file,
            line: diagnostic.line,
            column: diagnostic.column,
            message: diagnostic.message,
            hint: diagnostic.hint,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MachineEnvelope<T> {
    pub protocol_version: u32,
    pub command: String,
    pub ok: bool,
    pub data: Option<T>,
    pub diagnostics: Vec<MachineDiagnostic>,
    pub error: Option<MachineError>,
}

impl<T> MachineEnvelope<T> {
    #[must_use]
    pub fn success(command: impl Into<String>, data: T) -> Self {
        Self::completed(command, true, Some(data), Vec::new(), None)
    }

    #[must_use]
    pub fn failure(
        command: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::completed(
            command,
            false,
            None,
            Vec::new(),
            Some(MachineError::new(code, message)),
        )
    }

    #[must_use]
    pub fn completed(
        command: impl Into<String>,
        ok: bool,
        data: Option<T>,
        diagnostics: Vec<Diagnostic>,
        error: Option<MachineError>,
    ) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            command: command.into(),
            ok,
            data,
            diagnostics: diagnostics
                .into_iter()
                .map(MachineDiagnostic::from)
                .collect(),
            error,
        }
    }
}

/// Writes one complete protocol envelope followed by a newline.
///
/// # Errors
/// Returns serialization and stdout failures.
pub fn print<T: Serialize>(envelope: &MachineEnvelope<T>) -> Result<(), String> {
    let stdout = io::stdout();
    let mut output = stdout.lock();
    serde_json::to_writer_pretty(&mut output, envelope).map_err(|error| error.to_string())?;
    output.write_all(b"\n").map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_has_stable_top_level_shape() {
        let value = serde_json::to_value(MachineEnvelope::success(
            "check",
            serde_json::json!({"labels": 2}),
        ))
        .unwrap();
        assert_eq!(value["protocol_version"], PROTOCOL_VERSION);
        assert_eq!(value["command"], "check");
        assert_eq!(value["ok"], true);
        assert_eq!(value["data"]["labels"], 2);
        assert_eq!(value["diagnostics"], serde_json::json!([]));
        assert!(value["error"].is_null());
    }

    #[test]
    fn diagnostics_receive_machine_codes() {
        let envelope = MachineEnvelope::<serde_json::Value>::completed(
            "check",
            false,
            None,
            vec![Diagnostic::new("story.rns", 4, 2, "invalid")],
            Some(MachineError::new(code::VALIDATION_FAILED, "check failed")),
        );
        let value = serde_json::to_value(envelope).unwrap();
        assert_eq!(value["diagnostics"][0]["code"], "validation_error");
        assert_eq!(value["error"]["code"], code::VALIDATION_FAILED);
    }
}
