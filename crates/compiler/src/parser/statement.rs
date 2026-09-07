use super::{
    CropRect, Cursor, Diagnostic, Easing, Parser, Statement, StatementKind, TransformProperties,
    TransitionKind, TranslationId, parse_expression,
};

impl Parser {
    #[allow(clippy::too_many_lines)]
    pub(super) fn parse_statement(&mut self, indent: usize) -> Result<Statement, Diagnostic> {
        let line = self
            .peek()
            .cloned()
            .expect("parse_statement requires a current line");
        let span = self.span(line.number, indent + 1);
        let mut cursor = Cursor::new(&line.text);
        let mut id = None;
        let mut aliases = Vec::new();
        if cursor.consume_symbol('@') {
            if !cursor.keyword("id") {
                return Err(self.error(&line, cursor.column(), "expected `id` after `@`"));
            }
            let raw = cursor
                .string()
                .map_err(|message| self.error(&line, cursor.column(), message))?;
            id = Some(
                TranslationId::new(raw)
                    .map_err(|error| self.error(&line, cursor.column(), error.to_string()))?,
            );
            while cursor.keyword("alias") {
                let raw = cursor
                    .string()
                    .map_err(|message| self.error(&line, cursor.column(), message))?;
                aliases.push(
                    TranslationId::new(raw)
                        .map_err(|error| self.error(&line, cursor.column(), error.to_string()))?,
                );
            }
        }

        let kind = if cursor.keyword("extend") {
            let variable = cursor.identifier().ok_or_else(|| {
                self.error(&line, cursor.column(), "expected extension target variable")
            })?;
            cursor
                .symbol('=')
                .map_err(|error| self.error(&line, cursor.column(), error))?;
            let name = cursor
                .string()
                .map_err(|error| self.error(&line, cursor.column(), error))?;
            let column = cursor.column();
            let input = parse_expression(
                cursor.rest(),
                &self.source_name,
                line.number,
                indent + column,
            )?;
            self.current += 1;
            StatementKind::Extension {
                name,
                variable,
                input,
            }
        } else if cursor.keyword("nvl") {
            let mode = cursor.rest().trim();
            if !matches!(mode, "on" | "off" | "clear") {
                return Err(self.error(&line, cursor.column(), "expected nvl on, off or clear"));
            }
            self.current += 1;
            StatementKind::Nvl {
                mode: mode.to_owned(),
            }
        } else if cursor.keyword("parallel") {
            cursor
                .symbol(':')
                .map_err(|message| self.error(&line, cursor.column(), message))?;
            cursor
                .end()
                .map_err(|message| self.error(&line, cursor.column(), message))?;
            self.current += 1;
            let block = self.parse_block(indent + 4)?;
            super::parallel::parallel_tracks(block)
                .map_err(|message| self.error(&line, 1, message))?
        } else if cursor.keyword("timeline") {
            cursor
                .symbol(':')
                .map_err(|message| self.error(&line, cursor.column(), message))?;
            cursor
                .end()
                .map_err(|message| self.error(&line, cursor.column(), message))?;
            self.current += 1;
            let block = self.parse_block(indent + 4)?;
            if block.is_empty()
                || block.iter().any(|statement| {
                    !matches!(
                        statement.kind,
                        StatementKind::Transform { .. }
                            | StatementKind::Move { .. }
                            | StatementKind::Pause { .. }
                    )
                })
            {
                return Err(self.error(
                    &line,
                    1,
                    "timeline requires transform, move or pause keyframes",
                ));
            }
            StatementKind::Timeline { block }
        } else if cursor.keyword("video") {
            let path = cursor
                .string()
                .map_err(|message| self.error(&line, cursor.column(), message))?;
            if !cursor.keyword("over") {
                return Err(self.error(
                    &line,
                    cursor.column(),
                    "expected `over` and video duration",
                ));
            }
            let seconds = self.parse_duration(&line, cursor.rest())?;
            self.current += 1;
            StatementKind::Video { path, seconds }
        } else if cursor.keyword("scene") {
            self.parse_scene(&line, &mut cursor)?
        } else if cursor.keyword("show") {
            self.parse_show(&line, &mut cursor)?
        } else if cursor.keyword("hide") {
            self.parse_hide(&line, &mut cursor)?
        } else if cursor.keyword("clear") {
            self.parse_clear_layer(&line, &mut cursor)?
        } else if cursor.keyword("menu") {
            cursor
                .symbol(':')
                .map_err(|message| self.error(&line, cursor.column(), message))?;
            cursor
                .end()
                .map_err(|message| self.error(&line, cursor.column(), message))?;
            self.current += 1;
            StatementKind::Menu {
                options: self.parse_menu(indent + 4)?,
            }
        } else if cursor.keyword("jump") {
            let label = cursor
                .identifier()
                .ok_or_else(|| self.error(&line, cursor.column(), "expected label name"))?;
            cursor
                .end()
                .map_err(|message| self.error(&line, cursor.column(), message))?;
            self.current += 1;
            StatementKind::Jump { label }
        } else if cursor.keyword("call") {
            let label = cursor
                .identifier()
                .ok_or_else(|| self.error(&line, cursor.column(), "expected label name"))?;
            let arguments = if cursor.consume_symbol('(') {
                self.parse_argument_list(&line, indent, &mut cursor)?
            } else {
                Vec::new()
            };
            cursor
                .end()
                .map_err(|message| self.error(&line, cursor.column(), message))?;
            self.current += 1;
            StatementKind::Call { label, arguments }
        } else if cursor.keyword("return") {
            let column = cursor.column();
            let source = cursor.rest();
            let value = if source.is_empty() {
                None
            } else {
                Some(parse_expression(
                    source,
                    &self.source_name,
                    line.number,
                    indent + column,
                )?)
            };
            self.current += 1;
            StatementKind::Return { value }
        } else if cursor.keyword("set") {
            let variable = cursor
                .identifier()
                .ok_or_else(|| self.error(&line, cursor.column(), "expected variable name"))?;
            cursor
                .symbol('=')
                .map_err(|message| self.error(&line, cursor.column(), message))?;
            let expression_column = cursor.column();
            let expression_source = cursor.rest();
            if expression_source.is_empty() {
                return Err(self.error(&line, expression_column, "expected expression after `=`"));
            }
            let value = parse_expression(
                expression_source,
                &self.source_name,
                line.number,
                indent + expression_column,
            )?;
            self.current += 1;
            StatementKind::Set { variable, value }
        } else if cursor.keyword("if") {
            return self.parse_if(indent, &line, span, id, aliases, cursor);
        } else if cursor.keyword("play") {
            let target = cursor
                .identifier()
                .ok_or_else(|| self.error(&line, cursor.column(), "expected `music` or `sound`"))?;
            let path = cursor
                .string()
                .map_err(|message| self.error(&line, cursor.column(), message))?;
            let kind = match target.as_str() {
                "music" => {
                    let (repeat, fade_in, volume) = self.parse_music_options(&line, &mut cursor)?;
                    StatementKind::PlayMusic {
                        path,
                        repeat,
                        fade_in,
                        volume,
                    }
                }
                "sound" => {
                    let volume = if cursor.keyword("volume") {
                        self.parse_audio_volume(&line, &mut cursor)?
                    } else {
                        1.0
                    };
                    StatementKind::PlaySound { path, volume }
                }
                _ => return Err(self.error(&line, cursor.column(), "expected `music` or `sound`")),
            };
            cursor
                .end()
                .map_err(|message| self.error(&line, cursor.column(), message))?;
            self.current += 1;
            kind
        } else if cursor.keyword("queue") {
            if !cursor.keyword("music") {
                return Err(self.error(&line, cursor.column(), "only `queue music` is supported"));
            }
            let path = cursor
                .string()
                .map_err(|message| self.error(&line, cursor.column(), message))?;
            let (repeat, fade_in, volume) = self.parse_music_options(&line, &mut cursor)?;
            cursor
                .end()
                .map_err(|message| self.error(&line, cursor.column(), message))?;
            self.current += 1;
            StatementKind::QueueMusic {
                path,
                repeat,
                fade_in,
                volume,
            }
        } else if cursor.keyword("voice") {
            let path = cursor
                .string()
                .map_err(|message| self.error(&line, cursor.column(), message))?;
            cursor
                .end()
                .map_err(|message| self.error(&line, cursor.column(), message))?;
            self.current += 1;
            StatementKind::PlayVoice { path }
        } else if cursor.keyword("stop") {
            if !cursor.keyword("music") {
                return Err(self.error(&line, cursor.column(), "only `stop music` is supported"));
            }
            let fade_out = if cursor.keyword("fadeout") {
                self.parse_audio_duration(&line, &mut cursor, "fadeout")?
            } else {
                0.0
            };
            cursor
                .end()
                .map_err(|message| self.error(&line, cursor.column(), message))?;
            self.current += 1;
            StatementKind::StopMusic { fade_out }
        } else if cursor.keyword("pause") {
            let seconds = cursor.rest().parse::<f32>().map_err(|_| {
                self.error(&line, cursor.column(), "pause duration must be a number")
            })?;
            if !seconds.is_finite() || seconds < 0.0 {
                return Err(self.error(
                    &line,
                    cursor.column(),
                    "pause duration must be finite and non-negative",
                ));
            }
            self.current += 1;
            StatementKind::Pause { seconds }
        } else if cursor.keyword("move") {
            let alias = cursor
                .identifier()
                .ok_or_else(|| self.error(&line, cursor.column(), "expected image alias"))?;
            if !cursor.keyword("to") {
                return Err(self.error(&line, cursor.column(), "expected `to`"));
            }
            let position = self.parse_position(&line, &mut cursor)?;
            if !cursor.keyword("over") {
                return Err(self.error(&line, cursor.column(), "expected `over`"));
            }
            let seconds = self.parse_duration(&line, cursor.rest())?;
            self.current += 1;
            StatementKind::Move {
                alias,
                position,
                seconds,
            }
        } else if cursor.keyword("transform") {
            let alias = cursor
                .identifier()
                .ok_or_else(|| self.error(&line, cursor.column(), "expected image alias"))?;
            let mut properties = TransformProperties::default();
            let mut seconds = 0.0;
            let mut easing = Easing::Linear;
            let mut property_count = 0;
            while cursor.end().is_err() {
                if cursor.keyword("x") && properties.x.is_none() {
                    properties.x = Some(self.parse_transform_number(&line, &mut cursor, "x")?);
                    property_count += 1;
                } else if cursor.keyword("y") && properties.y.is_none() {
                    properties.y = Some(self.parse_transform_number(&line, &mut cursor, "y")?);
                    property_count += 1;
                } else if cursor.keyword("scale") && properties.scale.is_none() {
                    let value = self.parse_transform_number(&line, &mut cursor, "scale")?;
                    if !(0.01..=20.0).contains(&value) {
                        return Err(self.error(
                            &line,
                            cursor.column(),
                            "transform scale must be between 0.01 and 20",
                        ));
                    }
                    properties.scale = Some(value);
                    property_count += 1;
                } else if cursor.keyword("rotate") && properties.rotation.is_none() {
                    properties.rotation =
                        Some(self.parse_transform_number(&line, &mut cursor, "rotation")?);
                    property_count += 1;
                } else if cursor.keyword("alpha") && properties.alpha.is_none() {
                    let value = self.parse_transform_number(&line, &mut cursor, "alpha")?;
                    if !(0.0..=1.0).contains(&value) {
                        return Err(self.error(
                            &line,
                            cursor.column(),
                            "transform alpha must be between 0 and 1",
                        ));
                    }
                    properties.alpha = Some(value);
                    property_count += 1;
                } else if cursor.keyword("anchor") && properties.anchor.is_none() {
                    let x = self.parse_transform_number(&line, &mut cursor, "anchor x")?;
                    let y = self.parse_transform_number(&line, &mut cursor, "anchor y")?;
                    if !(0.0..=1.0).contains(&x) || !(0.0..=1.0).contains(&y) {
                        return Err(self.error(
                            &line,
                            cursor.column(),
                            "transform anchors must be between 0 and 1",
                        ));
                    }
                    properties.anchor = Some((x, y));
                    property_count += 1;
                } else if cursor.keyword("crop") && properties.crop.is_none() {
                    let crop = CropRect {
                        x: self.parse_transform_number(&line, &mut cursor, "crop x")?,
                        y: self.parse_transform_number(&line, &mut cursor, "crop y")?,
                        width: self.parse_transform_number(&line, &mut cursor, "crop width")?,
                        height: self.parse_transform_number(&line, &mut cursor, "crop height")?,
                    };
                    if crop.x < 0.0 || crop.y < 0.0 || crop.width <= 0.0 || crop.height <= 0.0 {
                        return Err(self.error(
                            &line,
                            cursor.column(),
                            "crop coordinates must be non-negative and its size must be positive",
                        ));
                    }
                    properties.crop = Some(Some(crop));
                    property_count += 1;
                } else if cursor.keyword("uncrop") && properties.crop.is_none() {
                    properties.crop = Some(None);
                    property_count += 1;
                } else if cursor.keyword("over") && seconds == 0.0 {
                    seconds = self.parse_transform_number(&line, &mut cursor, "duration")?;
                    if seconds < 0.0 {
                        return Err(self.error(
                            &line,
                            cursor.column(),
                            "transform duration must be non-negative",
                        ));
                    }
                } else if cursor.keyword("ease") && easing == Easing::Linear {
                    easing = match cursor.identifier().as_deref() {
                        Some("linear") => Easing::Linear,
                        Some("in") => Easing::EaseIn,
                        Some("out") => Easing::EaseOut,
                        Some("in_out") => Easing::EaseInOut,
                        _ => {
                            return Err(self.error(
                                &line,
                                cursor.column(),
                                "expected easing `linear`, `in`, `out`, or `in_out`",
                            ));
                        }
                    };
                } else {
                    return Err(self.error(
                        &line,
                        cursor.column(),
                        "expected transform property, `over`, or `ease`",
                    ));
                }
            }
            if property_count == 0 {
                return Err(self.error(
                    &line,
                    cursor.column(),
                    "transform requires at least one property",
                ));
            }
            if alias == "camera" && (properties.anchor.is_some() || properties.crop.is_some()) {
                return Err(self.error(&line, 1, "camera supports x, y, scale, rotate and alpha"));
            }
            self.current += 1;
            StatementKind::Transform {
                alias,
                properties,
                seconds,
                easing,
            }
        } else if cursor.keyword("transition") {
            let kind = match cursor.identifier().as_deref() {
                Some("fade") => TransitionKind::Fade,
                Some("dissolve") => TransitionKind::Dissolve,
                _ => {
                    return Err(self.error(
                        &line,
                        cursor.column(),
                        "expected `fade` or `dissolve`",
                    ));
                }
            };
            let seconds = self.parse_duration(&line, cursor.rest())?;
            self.current += 1;
            StatementKind::Transition { kind, seconds }
        } else {
            let speaker = cursor.identifier();
            let text = cursor
                .string()
                .map_err(|message| self.error(&line, cursor.column(), message))?;
            cursor
                .end()
                .map_err(|message| self.error(&line, cursor.column(), message))?;
            self.current += 1;
            StatementKind::Dialogue { speaker, text }
        };

        Ok(Statement {
            span,
            id,
            aliases,
            kind,
        })
    }
}
