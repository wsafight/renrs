use crate::syntax::{CallArgument, Expr, LabelParameter, Span};

use super::CompileError;

pub(super) fn bind_call_arguments(
    label: &str,
    expected: &[LabelParameter],
    supplied: &[CallArgument],
    span: &Span,
) -> Result<(Vec<Expr>, Vec<String>), CompileError> {
    let mut resolved = vec![None; expected.len()];
    let mut positional = 0_usize;
    for argument in supplied {
        let index = if let Some(name) = &argument.name {
            expected
                .iter()
                .position(|parameter| parameter.name == *name)
                .ok_or_else(|| CompileError::UnknownLabelArgument {
                    label: label.to_owned(),
                    argument: name.clone(),
                    file: span.source.clone(),
                    line: span.line,
                })?
        } else {
            if positional >= expected.len() {
                return Err(CompileError::TooManyLabelArguments {
                    label: label.to_owned(),
                    maximum: expected.len(),
                    found: positional + 1,
                    file: span.source.clone(),
                    line: span.line,
                });
            }
            let index = positional;
            positional += 1;
            index
        };
        if resolved[index].replace(argument.value.clone()).is_some() {
            return Err(CompileError::DuplicateLabelArgument {
                label: label.to_owned(),
                argument: expected[index].name.clone(),
                file: span.source.clone(),
                line: span.line,
            });
        }
    }

    let mut arguments = Vec::with_capacity(expected.len());
    let mut parameters = Vec::with_capacity(expected.len());
    for (parameter, value) in expected.iter().zip(resolved) {
        let value = value.or_else(|| parameter.default.clone()).ok_or_else(|| {
            CompileError::MissingLabelArgument {
                label: label.to_owned(),
                argument: parameter.name.clone(),
                file: span.source.clone(),
                line: span.line,
            }
        })?;
        parameters.push(parameter.name.clone());
        arguments.push(value);
    }
    Ok((arguments, parameters))
}
