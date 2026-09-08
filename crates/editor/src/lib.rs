pub mod lsp;
pub mod tooling;

pub(crate) use renrs_compiler::parser;
pub(crate) use renrs_syntax::{diagnostic, syntax, text};

#[cfg(test)]
pub(crate) use renrs_compiler::parse_script;
