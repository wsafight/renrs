use super::{Cursor, Diagnostic, Line, Parser, Position, StatementKind, default_alias};

impl Parser {
    pub(super) fn parse_scene(
        &mut self,
        line: &Line,
        cursor: &mut Cursor<'_>,
    ) -> Result<StatementKind, Diagnostic> {
        let path = self.parse_image_source(line, cursor)?;
        cursor
            .end()
            .map_err(|message| self.error(line, cursor.column(), message))?;
        self.current += 1;
        Ok(StatementKind::Scene { path })
    }

    pub(super) fn parse_show(
        &mut self,
        line: &Line,
        cursor: &mut Cursor<'_>,
    ) -> Result<StatementKind, Diagnostic> {
        let path = self.parse_image_source(line, cursor)?;
        let mut alias = path
            .strip_prefix("@image:")
            .map_or_else(|| default_alias(&path), ToOwned::to_owned);
        let mut position = Position::Center;
        let mut layer = 0;
        let mut display_layer = "master".to_owned();
        let mut at_transform = None;
        let (mut has_alias, mut has_position, mut has_zorder, mut has_display_layer) =
            (false, false, false, false);
        loop {
            if cursor.end().is_ok() {
                break;
            }
            if cursor.keyword("as") && !has_alias {
                alias = cursor.identifier().ok_or_else(|| {
                    self.error(line, cursor.column(), "expected alias after `as`")
                })?;
                has_alias = true;
            } else if cursor.keyword("at") && !has_position {
                let name = cursor.identifier().ok_or_else(|| {
                    self.error(line, cursor.column(), "expected position or transform name")
                })?;
                match name.as_str() {
                    "left" => position = Position::Left,
                    "center" => position = Position::Center,
                    "right" => position = Position::Right,
                    _ => at_transform = Some(name),
                }
                has_position = true;
            } else if (cursor.keyword("zorder") || cursor.keyword("layer")) && !has_zorder {
                layer = cursor
                    .integer()
                    .ok_or_else(|| self.error(line, cursor.column(), "expected integer z-order"))?;
                has_zorder = true;
            } else if cursor.keyword("onlayer") && !has_display_layer {
                display_layer = cursor.identifier().ok_or_else(|| {
                    self.error(line, cursor.column(), "expected display layer name")
                })?;
                has_display_layer = true;
            } else {
                return Err(self.error(
                    line,
                    cursor.column(),
                    "expected `as`, `at`, `zorder`, or `onlayer`",
                ));
            }
        }
        self.current += 1;
        Ok(StatementKind::Show {
            path,
            alias,
            position,
            layer,
            display_layer,
            at_transform,
        })
    }

    pub(super) fn parse_hide(
        &mut self,
        line: &Line,
        cursor: &mut Cursor<'_>,
    ) -> Result<StatementKind, Diagnostic> {
        let alias = cursor
            .identifier()
            .ok_or_else(|| self.error(line, cursor.column(), "expected image alias"))?;
        cursor
            .end()
            .map_err(|message| self.error(line, cursor.column(), message))?;
        self.current += 1;
        Ok(StatementKind::Hide { alias })
    }

    pub(super) fn parse_clear_layer(
        &mut self,
        line: &Line,
        cursor: &mut Cursor<'_>,
    ) -> Result<StatementKind, Diagnostic> {
        let display_layer = cursor
            .identifier()
            .ok_or_else(|| self.error(line, cursor.column(), "expected display layer name"))?;
        cursor
            .end()
            .map_err(|message| self.error(line, cursor.column(), message))?;
        self.current += 1;
        Ok(StatementKind::ClearLayer { display_layer })
    }
}
