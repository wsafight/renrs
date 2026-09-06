use super::{
    Cursor, Diagnostic, Line, MenuOption, Parser, Position, Span, Statement, StatementKind,
    TranslationId, parse_expression,
};

impl Parser {
    pub(super) fn parse_image_source(
        &self,
        line: &Line,
        cursor: &mut Cursor<'_>,
    ) -> Result<String, Diagnostic> {
        if cursor.peek_non_space() == Some('"') {
            cursor
                .string()
                .map_err(|message| self.error(line, cursor.column(), message))
        } else {
            cursor
                .identifier()
                .map(|name| format!("@image:{name}"))
                .ok_or_else(|| {
                    self.error(line, cursor.column(), "expected image name or quoted path")
                })
        }
    }

    pub(super) fn parse_music_options(
        &self,
        line: &Line,
        cursor: &mut Cursor<'_>,
    ) -> Result<(bool, f32), Diagnostic> {
        let mut repeat = false;
        let mut fade_in = 0.0;
        let mut has_fade = false;
        loop {
            if cursor.keyword("loop") && !repeat {
                repeat = true;
            } else if cursor.keyword("fadein") && !has_fade {
                fade_in = self.parse_audio_duration(line, cursor, "fadein")?;
                has_fade = true;
            } else {
                break;
            }
        }
        Ok((repeat, fade_in))
    }

    pub(super) fn parse_audio_duration(
        &self,
        line: &Line,
        cursor: &mut Cursor<'_>,
        name: &str,
    ) -> Result<f32, Diagnostic> {
        let value = cursor.number().ok_or_else(|| {
            self.error(
                line,
                cursor.column(),
                format!("{name} duration must be a number"),
            )
        })?;
        if value.is_finite() && value >= 0.0 {
            Ok(value)
        } else {
            Err(self.error(
                line,
                cursor.column(),
                format!("{name} duration must be finite and non-negative"),
            ))
        }
    }

    pub(super) fn parse_transform_number(
        &self,
        line: &Line,
        cursor: &mut Cursor<'_>,
        name: &str,
    ) -> Result<f32, Diagnostic> {
        cursor
            .number()
            .filter(|value| value.is_finite())
            .ok_or_else(|| {
                self.error(
                    line,
                    cursor.column(),
                    format!("transform {name} must be a finite number"),
                )
            })
    }

    pub(super) fn parse_argument_list(
        &self,
        line: &Line,
        indent: usize,
        cursor: &mut Cursor<'_>,
    ) -> Result<Vec<crate::syntax::Expr>, Diagnostic> {
        let mut arguments = Vec::new();
        if cursor.consume_symbol(')') {
            return Ok(arguments);
        }
        loop {
            let column = cursor.column();
            let source = cursor
                .argument_source()
                .map_err(|message| self.error(line, cursor.column(), message))?;
            arguments.push(parse_expression(
                source,
                &self.source_name,
                line.number,
                indent + column,
            )?);
            if cursor.consume_symbol(')') {
                break;
            }
            cursor
                .symbol(',')
                .map_err(|message| self.error(line, cursor.column(), message))?;
        }
        Ok(arguments)
    }

    pub(super) fn parse_position(
        &self,
        line: &Line,
        cursor: &mut Cursor<'_>,
    ) -> Result<Position, Diagnostic> {
        match cursor.identifier().as_deref() {
            Some("left") => Ok(Position::Left),
            Some("center") => Ok(Position::Center),
            Some("right") => Ok(Position::Right),
            _ => Err(self.error(
                line,
                cursor.column(),
                "expected `left`, `center`, or `right`",
            )),
        }
    }

    pub(super) fn parse_duration(&self, line: &Line, source: &str) -> Result<f32, Diagnostic> {
        let seconds = source
            .parse::<f32>()
            .map_err(|_| self.error(line, line.text.len(), "duration must be a number"))?;
        if !seconds.is_finite() || seconds < 0.0 {
            return Err(self.error(
                line,
                line.text.len(),
                "duration must be finite and non-negative",
            ));
        }
        Ok(seconds)
    }

    pub(super) fn parse_if(
        &mut self,
        indent: usize,
        line: &Line,
        span: Span,
        id: Option<TranslationId>,
        aliases: Vec<TranslationId>,
        mut cursor: Cursor<'_>,
    ) -> Result<Statement, Diagnostic> {
        let expression_column = cursor.column();
        let source = cursor
            .rest_before_colon()
            .map_err(|message| self.error(line, cursor.column(), message))?;
        let condition = parse_expression(
            source,
            &self.source_name,
            line.number,
            indent + expression_column,
        )?;
        self.current += 1;
        let body = self.parse_block(indent + 4)?;
        if body.is_empty() {
            return Err(self.error(line, 1, "if branch cannot be empty"));
        }
        let mut branches = vec![(condition, body)];
        let mut else_block = Vec::new();

        while let Some(next) = self.peek().cloned() {
            if next.indent != indent {
                break;
            }
            let mut branch_cursor = Cursor::new(&next.text);
            if branch_cursor.keyword("elif") {
                let column = branch_cursor.column();
                let source = branch_cursor
                    .rest_before_colon()
                    .map_err(|message| self.error(&next, branch_cursor.column(), message))?;
                let condition =
                    parse_expression(source, &self.source_name, next.number, indent + column)?;
                self.current += 1;
                let body = self.parse_block(indent + 4)?;
                if body.is_empty() {
                    return Err(self.error(&next, 1, "elif branch cannot be empty"));
                }
                branches.push((condition, body));
            } else if branch_cursor.keyword("else") {
                branch_cursor
                    .symbol(':')
                    .map_err(|message| self.error(&next, branch_cursor.column(), message))?;
                branch_cursor
                    .end()
                    .map_err(|message| self.error(&next, branch_cursor.column(), message))?;
                self.current += 1;
                else_block = self.parse_block(indent + 4)?;
                if else_block.is_empty() {
                    return Err(self.error(&next, 1, "else branch cannot be empty"));
                }
                break;
            } else {
                break;
            }
        }

        Ok(Statement {
            span,
            id,
            aliases,
            kind: StatementKind::If {
                branches,
                else_block,
            },
        })
    }

    pub(super) fn parse_menu(&mut self, indent: usize) -> Result<Vec<MenuOption>, Diagnostic> {
        let mut options = Vec::new();
        while let Some(line) = self.peek().cloned() {
            if line.indent < indent {
                break;
            }
            if line.indent > indent {
                return Err(self.error(&line, 1, "unexpected indentation in menu"));
            }
            let mut cursor = Cursor::new(&line.text);
            let text = cursor
                .string()
                .map_err(|message| self.error(&line, cursor.column(), message))?;
            let id = if cursor.keyword("id") {
                let raw = cursor
                    .string()
                    .map_err(|message| self.error(&line, cursor.column(), message))?;
                Some(
                    TranslationId::new(raw)
                        .map_err(|error| self.error(&line, cursor.column(), error.to_string()))?,
                )
            } else {
                None
            };
            let condition = if cursor.keyword("if") {
                let column = cursor.column();
                let source = cursor
                    .rest_before_colon()
                    .map_err(|message| self.error(&line, cursor.column(), message))?;
                Some(parse_expression(
                    source,
                    &self.source_name,
                    line.number,
                    indent + column,
                )?)
            } else {
                cursor
                    .symbol(':')
                    .map_err(|message| self.error(&line, cursor.column(), message))?;
                None
            };
            cursor
                .end()
                .map_err(|message| self.error(&line, cursor.column(), message))?;
            self.current += 1;
            let block = self.parse_block(indent + 4)?;
            if block.is_empty() {
                return Err(self.error(&line, 1, "menu option cannot have an empty block"));
            }
            options.push(MenuOption {
                text,
                block,
                span: self.span(line.number, indent + 1),
                id,
                condition,
            });
        }
        if options.len() < 2 {
            let line = self
                .lines
                .get(self.current.saturating_sub(1))
                .cloned()
                .unwrap_or(Line {
                    indent,
                    number: 1,
                    text: String::new(),
                });
            return Err(self
                .error(&line, 1, "menu must contain at least two options")
                .with_hint("add another quoted option at the same indentation"));
        }
        Ok(options)
    }

    pub(super) fn peek(&self) -> Option<&Line> {
        self.lines.get(self.current)
    }

    pub(super) fn error(
        &self,
        line: &Line,
        column: usize,
        message: impl Into<String>,
    ) -> Diagnostic {
        Diagnostic::new(
            &self.source_name,
            line.number,
            line.indent + column,
            message,
        )
    }

    pub(super) fn span(&self, line: usize, column: usize) -> Span {
        Span::in_source(&self.source_name, line, column)
    }
}
