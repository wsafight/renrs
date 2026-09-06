use crate::syntax::Value;
use crate::{Runtime, RuntimeError, WaitState};

impl Runtime {
    /// Evaluates extension input with the story expression parser.
    /// # Errors
    /// Reports invalid input expressions or extension failures.
    pub fn apply_extension_expression(
        &mut self,
        target: &str,
        name: &str,
        input: &str,
    ) -> Result<(), RuntimeError> {
        let expression = renrs_compiler::expression::parse_expression(input, "screens.json", 1, 1)
            .map_err(|error| RuntimeError::Execution {
                line: 0,
                message: error.to_string(),
            })?;
        let input = self.evaluate_expression(&expression)?;
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
