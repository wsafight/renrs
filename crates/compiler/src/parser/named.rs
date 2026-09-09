use super::{
    CropRect, Cursor, Diagnostic, Easing, Line, NamedTransform, Parser, TransformProperties,
};

impl Parser {
    pub(super) fn parse_named_transform(
        &mut self,
        line: &Line,
        cursor: &mut Cursor<'_>,
    ) -> Result<(String, NamedTransform), Diagnostic> {
        let name = cursor
            .identifier()
            .ok_or_else(|| self.error(line, cursor.column(), "expected transform name"))?;
        if matches!(name.as_str(), "left" | "center" | "right" | "camera") {
            return Err(self.error(
                line,
                cursor.column(),
                "transform name cannot be `left`, `center`, `right`, or `camera`",
            ));
        }
        let span = self.span(line.number, 1);
        if cursor.consume_symbol(':') {
            cursor
                .end()
                .map_err(|message| self.error(line, cursor.column(), message))?;
            self.current += 1;
            let properties = self.parse_named_transform_block(line.indent + 4)?;
            return Ok((name, NamedTransform { properties, span }));
        }
        let properties = self.parse_named_transform_properties(line, cursor)?;
        cursor
            .end()
            .map_err(|message| self.error(line, cursor.column(), message))?;
        self.current += 1;
        Ok((name, NamedTransform { properties, span }))
    }

    fn parse_named_transform_block(
        &mut self,
        indent: usize,
    ) -> Result<TransformProperties, Diagnostic> {
        let mut properties = TransformProperties::default();
        let mut any = false;
        while let Some(line) = self.peek().cloned() {
            if line.indent < indent {
                break;
            }
            if line.indent != indent {
                return Err(self.error(&line, 1, "unexpected indentation"));
            }
            let mut cursor = Cursor::new(&line.text);
            let parsed = self.parse_named_transform_properties(&line, &mut cursor)?;
            cursor
                .end()
                .map_err(|message| self.error(&line, cursor.column(), message))?;
            merge_properties(&mut properties, parsed)?;
            any = true;
            self.current += 1;
        }
        if any {
            Ok(properties)
        } else {
            Err(self.error(
                &self.lines[self.current.saturating_sub(1)].clone(),
                1,
                "named transform requires at least one property",
            ))
        }
    }

    fn parse_named_transform_properties(
        &self,
        line: &Line,
        cursor: &mut Cursor<'_>,
    ) -> Result<TransformProperties, Diagnostic> {
        let mut properties = TransformProperties::default();
        let mut count = 0;
        while cursor.end().is_err() {
            count += 1;
            if cursor.keyword("x") && properties.x.is_none() {
                properties.x = Some(self.parse_transform_number(line, cursor, "x")?);
            } else if cursor.keyword("y") && properties.y.is_none() {
                properties.y = Some(self.parse_transform_number(line, cursor, "y")?);
            } else if cursor.keyword("xpos") && properties.x.is_none() {
                properties.x = Some(self.parse_transform_number(line, cursor, "xpos")?);
            } else if cursor.keyword("ypos") && properties.y.is_none() {
                properties.y = Some(self.parse_transform_number(line, cursor, "ypos")?);
            } else if cursor.keyword("xalign") && properties.xalign.is_none() {
                let value = self.parse_transform_number(line, cursor, "xalign")?;
                if !(0.0..=1.0).contains(&value) {
                    return Err(self.error(
                        line,
                        cursor.column(),
                        "xalign must be between 0 and 1",
                    ));
                }
                properties.xalign = Some(value);
            } else if cursor.keyword("yalign") && properties.yalign.is_none() {
                let value = self.parse_transform_number(line, cursor, "yalign")?;
                if !(0.0..=1.0).contains(&value) {
                    return Err(self.error(
                        line,
                        cursor.column(),
                        "yalign must be between 0 and 1",
                    ));
                }
                properties.yalign = Some(value);
            } else if cursor.keyword("scale") && properties.scale.is_none() {
                properties.scale = Some(self.parse_transform_number(line, cursor, "scale")?);
            } else if cursor.keyword("rotate") && properties.rotation.is_none() {
                properties.rotation = Some(self.parse_transform_number(line, cursor, "rotation")?);
            } else if cursor.keyword("alpha") && properties.alpha.is_none() {
                let value = self.parse_transform_number(line, cursor, "alpha")?;
                if !(0.0..=1.0).contains(&value) {
                    return Err(self.error(line, cursor.column(), "alpha must be between 0 and 1"));
                }
                properties.alpha = Some(value);
            } else if cursor.keyword("anchor") && properties.anchor.is_none() {
                let x = self.parse_transform_number(line, cursor, "anchor x")?;
                let y = self.parse_transform_number(line, cursor, "anchor y")?;
                properties.anchor = Some((x, y));
            } else if cursor.keyword("crop") && properties.crop.is_none() {
                properties.crop = Some(Some(CropRect {
                    x: self.parse_transform_number(line, cursor, "crop x")?,
                    y: self.parse_transform_number(line, cursor, "crop y")?,
                    width: self.parse_transform_number(line, cursor, "crop width")?,
                    height: self.parse_transform_number(line, cursor, "crop height")?,
                }));
            } else if cursor.keyword("uncrop") && properties.crop.is_none() {
                properties.crop = Some(None);
            } else if cursor.keyword("ease") {
                let _ = match cursor.identifier().as_deref() {
                    Some("linear") => Easing::Linear,
                    Some("in") => Easing::EaseIn,
                    Some("out") => Easing::EaseOut,
                    Some("in_out") => Easing::EaseInOut,
                    _ => return Err(self.error(line, cursor.column(), "expected easing name")),
                };
            } else {
                return Err(self.error(line, cursor.column(), "expected transform property"));
            }
        }
        if count == 0 {
            Err(self.error(line, cursor.column(), "named transform requires a property"))
        } else {
            Ok(properties)
        }
    }
}

fn merge_properties(
    into: &mut TransformProperties,
    from: TransformProperties,
) -> Result<(), Diagnostic> {
    merge_one(&mut into.x, from.x, "x")?;
    merge_one(&mut into.y, from.y, "y")?;
    merge_one(&mut into.scale, from.scale, "scale")?;
    merge_one(&mut into.rotation, from.rotation, "rotate")?;
    merge_one(&mut into.alpha, from.alpha, "alpha")?;
    merge_one(&mut into.xalign, from.xalign, "xalign")?;
    merge_one(&mut into.yalign, from.yalign, "yalign")?;
    if from.anchor.is_some() {
        if into.anchor.is_some() {
            return Err(Diagnostic::new(
                ".",
                1,
                1,
                "duplicate transform property `anchor`",
            ));
        }
        into.anchor = from.anchor;
    }
    if from.crop.is_some() {
        if into.crop.is_some() {
            return Err(Diagnostic::new(
                ".",
                1,
                1,
                "duplicate transform property `crop`",
            ));
        }
        into.crop = from.crop;
    }
    Ok(())
}

fn merge_one(slot: &mut Option<f32>, value: Option<f32>, name: &str) -> Result<(), Diagnostic> {
    if value.is_none() {
        return Ok(());
    }
    if slot.is_some() {
        return Err(Diagnostic::new(
            ".",
            1,
            1,
            format!("duplicate transform property `{name}`"),
        ));
    }
    *slot = value;
    Ok(())
}
