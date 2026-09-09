use super::{Cursor, Diagnostic, Line, Parser, StatementKind, TransitionKind};

impl Parser {
    pub(super) fn parse_window(
        &mut self,
        line: &Line,
        cursor: &mut Cursor<'_>,
    ) -> Result<StatementKind, Diagnostic> {
        let visible = if cursor.keyword("show") {
            true
        } else if cursor.keyword("hide") {
            false
        } else {
            return Err(self.error(line, cursor.column(), "expected `show` or `hide`"));
        };
        cursor
            .end()
            .map_err(|message| self.error(line, cursor.column(), message))?;
        self.current += 1;
        Ok(StatementKind::Window { visible })
    }

    pub(super) fn parse_repeat(
        &mut self,
        line: &Line,
        cursor: &mut Cursor<'_>,
    ) -> Result<StatementKind, Diagnostic> {
        let count = cursor
            .integer()
            .ok_or_else(|| self.error(line, cursor.column(), "expected repeat count"))?;
        if !(1..=16).contains(&count) {
            return Err(self.error(line, cursor.column(), "repeat count must be 1..16"));
        }
        cursor
            .end()
            .map_err(|message| self.error(line, cursor.column(), message))?;
        self.current += 1;
        Ok(StatementKind::Repeat {
            count: u32::try_from(count).unwrap_or(1),
        })
    }

    pub(super) fn parse_transition_kind(
        &self,
        line: &Line,
        cursor: &mut Cursor<'_>,
    ) -> Result<TransitionKind, Diagnostic> {
        let name = cursor
            .identifier()
            .ok_or_else(|| self.error(line, cursor.column(), "expected transition kind"))?;
        Ok(match name.as_str() {
            "fade" => TransitionKind::Fade,
            "dissolve" => TransitionKind::Dissolve,
            "push" => match cursor.identifier().as_deref() {
                Some("left") => TransitionKind::PushLeft,
                Some("right") => TransitionKind::PushRight,
                _ => {
                    return Err(self.error(line, cursor.column(), "expected `left` or `right`"));
                }
            },
            "wipe" => match cursor.identifier().as_deref() {
                Some("left") => TransitionKind::WipeLeft,
                Some("right") => TransitionKind::WipeRight,
                _ => {
                    return Err(self.error(line, cursor.column(), "expected `left` or `right`"));
                }
            },
            "punch" => match cursor.identifier().as_deref() {
                Some("h") => TransitionKind::PunchH,
                Some("v") => TransitionKind::PunchV,
                _ => return Err(self.error(line, cursor.column(), "expected `h` or `v`")),
            },
            _ => {
                return Err(self.error(
                    line,
                    cursor.column(),
                    "expected `fade`, `dissolve`, `push`, `wipe`, or `punch`",
                ));
            }
        })
    }

    pub(super) fn parse_dialogue(
        &mut self,
        line: &Line,
        cursor: &mut Cursor<'_>,
        speaker: Option<String>,
    ) -> Result<StatementKind, Diagnostic> {
        let mut attributes = Vec::new();
        if speaker.is_some() {
            while cursor.peek_non_space() != Some('"') {
                let Some(attribute) = cursor.identifier() else {
                    break;
                };
                if attributes.len() >= 8 {
                    return Err(self.error(
                        line,
                        cursor.column(),
                        "dialogue supports at most 8 attributes",
                    ));
                }
                attributes.push(attribute);
            }
        }
        let text = cursor
            .string()
            .map_err(|message| self.error(line, cursor.column(), message))?;
        cursor
            .end()
            .map_err(|message| self.error(line, cursor.column(), message))?;
        self.current += 1;
        Ok(StatementKind::Dialogue {
            speaker,
            attributes,
            text,
        })
    }
}
