use crate::syntax::{Expr, Value};
use crate::{Runtime, RuntimeError, WaitState};

impl Runtime {
    /// Evaluates extension input with the story expression parser.
    /// # Errors
    /// Reports invalid input expressions or extension failures.
    pub fn apply_extension_expression(
        &mut self,
        target: &str,
        name: &str,
        input: &Expr,
    ) -> Result<(), RuntimeError> {
        let input = self.evaluate_expression(input)?;
        self.apply_extension(target, name, &input)
    }
    /// Runs a project extension as a pure function without changing story state.
    /// # Errors
    /// Reports missing modules, invalid data or execution limits.
    pub fn invoke_extension(&self, name: &str, input: &Value) -> Result<Value, RuntimeError> {
        self.extensions
            .invoke(name, input)
            .map_err(|message| RuntimeError::Execution { line: 0, message })
    }

    /// Applies an extension result as a transactional screen update.
    /// # Errors
    /// Rejects non-interactive states, failed scripts and invalid variable updates.
    pub fn apply_extension(
        &mut self,
        target: &str,
        name: &str,
        input: &Value,
    ) -> Result<(), RuntimeError> {
        if !matches!(
            self.waiting(),
            Some(WaitState::Dialogue | WaitState::Choice { .. })
        ) {
            return Err(RuntimeError::Execution {
                line: 0,
                message: "extensions require a dialogue or choice".to_owned(),
            });
        }
        let output = self.invoke_extension(name, input)?;
        self.set_screen_variable(target, output)
    }
}
