use renrs_model::InstructionId;
use renrs_syntax::localization::TranslationId;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("unknown label `{label}` referenced at {file}:{line}")]
    UnknownLabel {
        label: String,
        file: String,
        line: usize,
    },
    #[error("stable instruction id collision `{0}")]
    DuplicateInstructionId(InstructionId),
    #[error("translation id `{0}` is used more than once")]
    DuplicateTranslationId(TranslationId),
    #[error("unknown image `{name}` referenced at {file}:{line}")]
    UnknownImage {
        name: String,
        file: String,
        line: usize,
    },
    #[error("unknown display layer `{name}` referenced at {file}:{line}")]
    UnknownDisplayLayer {
        name: String,
        file: String,
        line: usize,
    },
    #[error(
        "label `{label}` accepts at most {maximum} positional arguments but received {found} at {file}:{line}"
    )]
    TooManyLabelArguments {
        label: String,
        maximum: usize,
        found: usize,
        file: String,
        line: usize,
    },
    #[error("label `{label}` has no parameter named `{argument}` at {file}:{line}")]
    UnknownLabelArgument {
        label: String,
        argument: String,
        file: String,
        line: usize,
    },
    #[error("label `{label}` receives parameter `{argument}` more than once at {file}:{line}")]
    DuplicateLabelArgument {
        label: String,
        argument: String,
        file: String,
        line: usize,
    },
    #[error("label `{label}` is missing required argument `{argument}` at {file}:{line}")]
    MissingLabelArgument {
        label: String,
        argument: String,
        file: String,
        line: usize,
    },
    #[error("entry label `start` cannot declare parameters at {file}:{line}")]
    ParameterizedStart { file: String, line: usize },
    #[error("jump cannot enter parameterized label `{label}` at {file}:{line}; use `call`")]
    ParameterizedJump {
        label: String,
        file: String,
        line: usize,
    },
    #[error("failed to fingerprint script: {0}")]
    Fingerprint(serde_json::Error),
}
