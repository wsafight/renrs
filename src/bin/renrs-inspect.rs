use std::path::PathBuf;
use std::process::ExitCode;

use renrs::inspection::inspect_project;
use renrs::protocol::{self, MachineEnvelope, MachineError};

fn main() -> ExitCode {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    let [project] = arguments.as_slice() else {
        let envelope = MachineEnvelope::<serde_json::Value>::failure(
            "inspect",
            protocol::code::INVALID_ARGUMENTS,
            "usage: renrs-inspect <project|archive>",
        );
        let _ = protocol::print(&envelope);
        return ExitCode::from(protocol::EXIT_USAGE);
    };
    match inspect_project(&PathBuf::from(project)) {
        Ok((inspection, diagnostics)) => {
            let envelope =
                MachineEnvelope::completed("inspect", true, Some(inspection), diagnostics, None);
            if let Err(error) = protocol::print(&envelope) {
                eprintln!("error: {error}");
                ExitCode::from(protocol::EXIT_FAILURE)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(failure) => {
            let envelope = MachineEnvelope::<serde_json::Value>::completed(
                "inspect",
                false,
                None,
                failure.diagnostics,
                Some(MachineError::new(failure.code, failure.message)),
            );
            let _ = protocol::print(&envelope);
            ExitCode::from(protocol::EXIT_FAILURE)
        }
    }
}
