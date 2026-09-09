use crate::diagnostic::Diagnostic;
use crate::syntax::{BinaryOp, Builtin, Expr, UnaryOp, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
enum TokenKind {
    Integer(i64),
    Boolean(bool),
    String(String),
    Identifier(String),
    LeftParen,
    RightParen,
    Comma,
    Plus,
    Minus,
    Star,
    Slash,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    Not,
    End,
}

#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    column: usize,
}

/// Parses a bounded story expression at its source location.
/// # Errors
/// Returns a diagnostic for invalid syntax.
pub fn parse_expression(
    input: &str,
    file: &str,
    line: usize,
    base_column: usize,
) -> Result<Expr, Diagnostic> {
    if input.len() > 65536 {
        return Err(Diagnostic::new(
            file,
            line,
            base_column,
            "expression exceeds 64 KiB",
        ));
    }
    let tokens = Lexer::new(input, file, line, base_column).lex()?;
    if tokens.len() > 512 {
        return Err(Diagnostic::new(
            file,
            line,
            base_column,
            "expression exceeds 512 tokens",
        ));
    }
    let mut depth: usize = 0;
    for token in &tokens {
        if token.kind == TokenKind::LeftParen {
            depth += 1;
        }
        if depth > 32 {
            return Err(Diagnostic::new(
                file,
                line,
                token.column,
                "expression nesting exceeds 32",
            ));
        }
        if token.kind == TokenKind::RightParen {
            depth = depth.saturating_sub(1);
        }
    }
    let mut parser = ExpressionParser {
        tokens,
        current: 0,
        file,
        line,
    };
    let expression = parser.parse_or()?;
    if parser.peek().kind != TokenKind::End {
        return Err(parser.error("unexpected token after expression"));
    }
    Ok(expression)
}

struct Lexer<'a> {
    input: &'a str,
    file: &'a str,
    line: usize,
    base_column: usize,
    offset: usize,
}

impl<'a> Lexer<'a> {
    const fn new(input: &'a str, file: &'a str, line: usize, base_column: usize) -> Self {
        Self {
            input,
            file,
            line,
            base_column,
            offset: 0,
        }
    }

    fn lex(mut self) -> Result<Vec<Token>, Diagnostic> {
        let mut tokens = Vec::new();
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace() {
                self.bump();
                continue;
            }

            let column = self.column();
            let kind = match ch {
                '(' => {
                    self.bump();
                    TokenKind::LeftParen
                }
                ')' => {
                    self.bump();
                    TokenKind::RightParen
                }
                ',' => {
                    self.bump();
                    TokenKind::Comma
                }
                '+' => {
                    self.bump();
                    TokenKind::Plus
                }
                '-' => {
                    self.bump();
                    TokenKind::Minus
                }
                '*' => {
                    self.bump();
                    TokenKind::Star
                }
                '/' => {
                    self.bump();
                    TokenKind::Slash
                }
                '=' => {
                    self.bump();
                    if self.peek_char() == Some('=') {
                        self.bump();
                        TokenKind::Equal
                    } else {
                        return Err(self.error_at(column, "expected `==` in expression"));
                    }
                }
                '!' => {
                    self.bump();
                    if self.peek_char() == Some('=') {
                        self.bump();
                        TokenKind::NotEqual
                    } else {
                        return Err(self.error_at(column, "expected `!=` in expression"));
                    }
                }
                '<' => {
                    self.bump();
                    if self.peek_char() == Some('=') {
                        self.bump();
                        TokenKind::LessEqual
                    } else {
                        TokenKind::Less
                    }
                }
                '>' => {
                    self.bump();
                    if self.peek_char() == Some('=') {
                        self.bump();
                        TokenKind::GreaterEqual
                    } else {
                        TokenKind::Greater
                    }
                }
                '"' => TokenKind::String(self.string()?),
                c if c.is_ascii_digit() => TokenKind::Integer(self.integer()?),
                c if is_identifier_start(c) => {
                    let word = self.identifier();
                    match word.as_str() {
                        "true" => TokenKind::Boolean(true),
                        "false" => TokenKind::Boolean(false),
                        "and" => TokenKind::And,
                        "or" => TokenKind::Or,
                        "not" => TokenKind::Not,
                        _ => TokenKind::Identifier(word),
                    }
                }
                _ => {
                    return Err(self.error_at(column, format!("unexpected character `{ch}`")));
                }
            };
            tokens.push(Token { kind, column });
        }
        tokens.push(Token {
            kind: TokenKind::End,
            column: self.column(),
        });
        Ok(tokens)
    }

    fn string(&mut self) -> Result<String, Diagnostic> {
        let start = self.column();
        self.bump();
        let mut value = String::new();
        while let Some(ch) = self.bump() {
            match ch {
                '"' => return Ok(value),
                '\\' => {
                    let escaped = self
                        .bump()
                        .ok_or_else(|| self.error_at(start, "unterminated string literal"))?;
                    match escaped {
                        'n' => value.push('\n'),
                        'r' => value.push('\r'),
                        't' => value.push('\t'),
                        '"' => value.push('"'),
                        '\\' => value.push('\\'),
                        _ => {
                            return Err(self.error_at(
                                self.column().saturating_sub(1),
                                format!("unsupported escape `\\{escaped}`"),
                            ));
                        }
                    }
                }
                _ => value.push(ch),
            }
        }
        Err(self.error_at(start, "unterminated string literal"))
    }

    fn integer(&mut self) -> Result<i64, Diagnostic> {
        let start = self.offset;
        while self.peek_char().is_some_and(|ch| ch.is_ascii_digit()) {
            self.bump();
        }
        self.input[start..self.offset]
            .parse()
            .map_err(|_| self.error_at(self.base_column + start, "integer is out of range"))
    }

    fn identifier(&mut self) -> String {
        let start = self.offset;
        while self.peek_char().is_some_and(is_identifier_continue) {
            self.bump();
        }
        self.input[start..self.offset].to_owned()
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.offset..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.offset += ch.len_utf8();
        Some(ch)
    }

    const fn column(&self) -> usize {
        self.base_column + self.offset
    }

    fn error_at(&self, column: usize, message: impl Into<String>) -> Diagnostic {
        Diagnostic::new(self.file, self.line, column, message)
    }
}

struct ExpressionParser<'a> {
    tokens: Vec<Token>,
    current: usize,
    file: &'a str,
    line: usize,
}

impl ExpressionParser<'_> {
    fn parse_or(&mut self) -> Result<Expr, Diagnostic> {
        let mut expression = self.parse_and()?;
        while self.consume(&TokenKind::Or) {
            expression = binary(expression, BinaryOp::Or, self.parse_and()?);
        }
        Ok(expression)
    }

    fn parse_and(&mut self) -> Result<Expr, Diagnostic> {
        let mut expression = self.parse_equality()?;
        while self.consume(&TokenKind::And) {
            expression = binary(expression, BinaryOp::And, self.parse_equality()?);
        }
        Ok(expression)
    }

    fn parse_equality(&mut self) -> Result<Expr, Diagnostic> {
        let mut expression = self.parse_comparison()?;
        loop {
            let op = if self.consume(&TokenKind::Equal) {
                Some(BinaryOp::Equal)
            } else if self.consume(&TokenKind::NotEqual) {
                Some(BinaryOp::NotEqual)
            } else {
                None
            };
            let Some(op) = op else { break };
            expression = binary(expression, op, self.parse_comparison()?);
        }
        Ok(expression)
    }

    fn parse_comparison(&mut self) -> Result<Expr, Diagnostic> {
        let mut expression = self.parse_term()?;
        loop {
            let op = if self.consume(&TokenKind::Less) {
                Some(BinaryOp::Less)
            } else if self.consume(&TokenKind::LessEqual) {
                Some(BinaryOp::LessEqual)
            } else if self.consume(&TokenKind::Greater) {
                Some(BinaryOp::Greater)
            } else if self.consume(&TokenKind::GreaterEqual) {
                Some(BinaryOp::GreaterEqual)
            } else {
                None
            };
            let Some(op) = op else { break };
            expression = binary(expression, op, self.parse_term()?);
        }
        Ok(expression)
    }

    fn parse_term(&mut self) -> Result<Expr, Diagnostic> {
        let mut expression = self.parse_factor()?;
        loop {
            let op = if self.consume(&TokenKind::Plus) {
                Some(BinaryOp::Add)
            } else if self.consume(&TokenKind::Minus) {
                Some(BinaryOp::Subtract)
            } else {
                None
            };
            let Some(op) = op else { break };
            expression = binary(expression, op, self.parse_factor()?);
        }
        Ok(expression)
    }

    fn parse_factor(&mut self) -> Result<Expr, Diagnostic> {
        let mut expression = self.parse_unary()?;
        loop {
            let op = if self.consume(&TokenKind::Star) {
                Some(BinaryOp::Multiply)
            } else if self.consume(&TokenKind::Slash) {
                Some(BinaryOp::Divide)
            } else {
                None
            };
            let Some(op) = op else { break };
            expression = binary(expression, op, self.parse_unary()?);
        }
        Ok(expression)
    }

    fn parse_unary(&mut self) -> Result<Expr, Diagnostic> {
        if self.consume(&TokenKind::Minus) {
            return Ok(Expr::Unary {
                op: UnaryOp::Negate,
                value: Box::new(self.parse_unary()?),
            });
        }
        if self.consume(&TokenKind::Not) {
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                value: Box::new(self.parse_unary()?),
            });
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, Diagnostic> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Integer(value) => Ok(Expr::Value(Value::Integer(value))),
            TokenKind::Boolean(value) => Ok(Expr::Value(Value::Boolean(value))),
            TokenKind::String(value) => Ok(Expr::Value(Value::String(value))),
            TokenKind::Identifier(value) if self.consume(&TokenKind::LeftParen) => {
                let function = Builtin::named(&value)
                    .ok_or_else(|| self.error(format!("unknown built-in function `{value}`")))?;
                let mut arguments = Vec::new();
                if !self.consume(&TokenKind::RightParen) {
                    loop {
                        arguments.push(self.parse_or()?);
                        if self.consume(&TokenKind::RightParen) {
                            break;
                        }
                        if !self.consume(&TokenKind::Comma) {
                            return Err(self.error("expected `,` or `)`"));
                        }
                    }
                }
                if !function.accepts(arguments.len()) {
                    return Err(self.error(format!("invalid argument count for `{value}`")));
                }
                Ok(Expr::Invoke {
                    function,
                    arguments,
                })
            }
            TokenKind::Identifier(value) => Ok(Expr::Variable(value)),
            TokenKind::LeftParen => {
                let expression = self.parse_or()?;
                if !self.consume(&TokenKind::RightParen) {
                    return Err(self.error("expected `)`"));
                }
                Ok(expression)
            }
            _ => Err(Diagnostic::new(
                self.file,
                self.line,
                token.column,
                "expected a value, variable, or parenthesized expression",
            )),
        }
    }

    fn consume(&mut self, kind: &TokenKind) -> bool {
        if &self.peek().kind == kind {
            self.current += 1;
            true
        } else {
            false
        }
    }

    fn advance(&mut self) -> &Token {
        let index = self.current;
        if self.tokens[index].kind != TokenKind::End {
            self.current += 1;
        }
        &self.tokens[index]
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn error(&self, message: impl Into<String>) -> Diagnostic {
        Diagnostic::new(self.file, self.line, self.peek().column, message)
    }
}

fn binary(left: Expr, op: BinaryOp, right: Expr) -> Expr {
    Expr::Binary {
        left: Box::new(left),
        op,
        right: Box::new(right),
    }
}

const fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

const fn is_identifier_continue(ch: char) -> bool {
    is_identifier_start(ch) || ch.is_ascii_digit()
}

#[cfg(test)]
#[path = "expression/tests.rs"]
mod tests;
