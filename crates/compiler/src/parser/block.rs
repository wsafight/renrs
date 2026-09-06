use super::{Block, Diagnostic, Parser, starts_keyword};

impl Parser {
    pub(super) fn parse_block(&mut self, indent: usize) -> Result<Block, Diagnostic> {
        let mut statements = Vec::new();
        while let Some(line) = self.peek().cloned() {
            if line.indent < indent {
                break;
            }
            if line.indent > indent {
                return Err(self
                    .error(&line, 1, "unexpected indentation")
                    .with_hint(format!("expected exactly {indent} leading spaces")));
            }
            if starts_keyword(&line.text, "elif") || starts_keyword(&line.text, "else") {
                break;
            }
            statements.push(self.parse_statement(indent)?);
        }
        Ok(statements)
    }
}
