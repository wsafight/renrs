use super::{
    CharacterDef, ConfigDeclaration, Cursor, DefaultDef, Diagnostic, DisplayLayerDef, ImageDef,
    IndexMap, LabelParameter, Line, Parser, ScriptFragment, parse_expression, valid_project_id,
};

impl Parser {
    #[allow(clippy::too_many_lines)]
    pub(super) fn parse(&mut self) -> Result<ScriptFragment, Vec<Diagnostic>> {
        let mut title = None;
        let mut project_id = None;
        let mut characters = IndexMap::new();
        let mut defaults = IndexMap::new();
        let mut images = IndexMap::new();
        let mut display_layers = IndexMap::new();
        let mut transforms = IndexMap::new();
        let mut label_parameters = IndexMap::new();
        let mut labels = IndexMap::new();
        let mut errors = Vec::new();

        while let Some(line) = self.peek().cloned() {
            if line.indent != 0 {
                errors.push(self.error(&line, 1, "top-level declarations must not be indented"));
                self.current += 1;
                continue;
            }

            let mut cursor = Cursor::new(&line.text);
            if cursor.keyword("config") {
                match self.parse_config(&line, &mut cursor) {
                    Ok(ConfigDeclaration::Title(value)) => {
                        if title.replace((value, self.span(line.number, 1))).is_some() {
                            errors.push(self.error(
                                &line,
                                1,
                                "`config title` is declared more than once",
                            ));
                        }
                        self.current += 1;
                    }
                    Ok(ConfigDeclaration::Id(value)) => {
                        if project_id
                            .replace((value, self.span(line.number, 1)))
                            .is_some()
                        {
                            errors.push(self.error(
                                &line,
                                1,
                                "`config id` is declared more than once",
                            ));
                        }
                        self.current += 1;
                    }
                    Err(error) => {
                        errors.push(error);
                        self.current += 1;
                    }
                }
            } else if cursor.keyword("default") {
                match self.parse_default(&line, &mut cursor) {
                    Ok((name, definition)) => {
                        if defaults.insert(name.clone(), definition).is_some() {
                            errors.push(self.error(
                                &line,
                                1,
                                format!("default `{name}` is declared more than once"),
                            ));
                        }
                        self.current += 1;
                    }
                    Err(error) => {
                        errors.push(error);
                        self.current += 1;
                    }
                }
            } else if cursor.keyword("image") {
                match self.parse_image(&line, &mut cursor) {
                    Ok((name, definition)) => {
                        if images.insert(name.clone(), definition).is_some() {
                            errors.push(self.error(
                                &line,
                                1,
                                format!("image `{name}` is declared more than once"),
                            ));
                        }
                        self.current += 1;
                    }
                    Err(error) => {
                        errors.push(error);
                        self.current += 1;
                    }
                }
            } else if cursor.keyword("layer") {
                match self.parse_display_layer(&line, &mut cursor) {
                    Ok((name, definition)) => {
                        if display_layers.insert(name.clone(), definition).is_some() {
                            errors.push(self.error(
                                &line,
                                1,
                                format!("display layer `{name}` is declared more than once"),
                            ));
                        }
                        self.current += 1;
                    }
                    Err(error) => {
                        errors.push(error);
                        self.current += 1;
                    }
                }
            } else if cursor.keyword("transform") {
                match self.parse_named_transform(&line, &mut cursor) {
                    Ok((name, definition)) => {
                        if transforms.insert(name.clone(), definition).is_some() {
                            errors.push(self.error(
                                &line,
                                1,
                                format!("transform `{name}` is declared more than once"),
                            ));
                        }
                    }
                    Err(error) => {
                        errors.push(error);
                        self.current += 1;
                    }
                }
            } else if cursor.keyword("define") {
                match self.parse_character(&line, &mut cursor) {
                    Ok((id, character)) => {
                        if characters.insert(id.clone(), character).is_some() {
                            errors.push(self.error(
                                &line,
                                1,
                                format!("character `{id}` is defined more than once"),
                            ));
                        }
                        self.current += 1;
                    }
                    Err(error) => {
                        errors.push(error);
                        self.current += 1;
                    }
                }
            } else if cursor.keyword("label") {
                match self.parse_label_header(&line, &mut cursor) {
                    Ok((name, parameters)) => {
                        self.current += 1;
                        match self.parse_block(4) {
                            Ok(block) if block.is_empty() => errors.push(self.error(
                                &line,
                                line.text.len(),
                                format!("label `{name}` has an empty block"),
                            )),
                            Ok(block) => {
                                if labels.insert(name.clone(), block).is_some() {
                                    errors.push(self.error(
                                        &line,
                                        1,
                                        format!("label `{name}` is defined more than once"),
                                    ));
                                } else {
                                    label_parameters.insert(name, parameters);
                                }
                            }
                            Err(error) => errors.push(error),
                        }
                    }
                    Err(error) => {
                        errors.push(error);
                        self.current += 1;
                    }
                }
            } else {
                errors.push(
                    self.error(
                        &line,
                        1,
                        "expected `config`, `default`, `image`, `layer`, `transform`, `define`, or `label`",
                    )
                    .with_hint("executable statements belong inside a label block"),
                );
                self.current += 1;
            }
        }

        if errors.is_empty() {
            Ok(ScriptFragment {
                source_name: self.source_name.clone(),
                title,
                project_id,
                characters,
                defaults,
                images,
                display_layers,
                transforms,
                label_parameters,
                labels,
            })
        } else {
            Err(errors)
        }
    }

    fn parse_config(
        &self,
        line: &Line,
        cursor: &mut Cursor<'_>,
    ) -> Result<ConfigDeclaration, Diagnostic> {
        let kind = cursor
            .identifier()
            .ok_or_else(|| self.error(line, cursor.column(), "expected `title` or `id`"))?;
        let value = cursor
            .string()
            .map_err(|message| self.error(line, cursor.column(), message))?;
        cursor
            .end()
            .map_err(|message| self.error(line, cursor.column(), message))?;
        match kind.as_str() {
            "title" => Ok(ConfigDeclaration::Title(value)),
            "id" if valid_project_id(&value) => Ok(ConfigDeclaration::Id(value)),
            "id" => Err(self.error(
                line,
                1,
                "project id must be a safe ASCII reverse-DNS name or slug",
            )),
            _ => Err(self.error(line, 1, "only `config title` and `config id` are supported")),
        }
    }

    fn parse_default(
        &self,
        line: &Line,
        cursor: &mut Cursor<'_>,
    ) -> Result<(String, DefaultDef), Diagnostic> {
        let name = cursor
            .identifier()
            .ok_or_else(|| self.error(line, cursor.column(), "expected variable name"))?;
        cursor
            .symbol('=')
            .map_err(|message| self.error(line, cursor.column(), message))?;
        let column = cursor.column();
        let source = cursor.rest();
        if source.is_empty() {
            return Err(self.error(line, column, "expected expression after `=`"));
        }
        let value = parse_expression(source, &self.source_name, line.number, line.indent + column)?;
        Ok((
            name,
            DefaultDef {
                value,
                span: self.span(line.number, 1),
            },
        ))
    }

    fn parse_image(
        &self,
        line: &Line,
        cursor: &mut Cursor<'_>,
    ) -> Result<(String, ImageDef), Diagnostic> {
        let name = cursor
            .identifier()
            .ok_or_else(|| self.error(line, cursor.column(), "expected image name"))?;
        cursor
            .symbol('=')
            .map_err(|message| self.error(line, cursor.column(), message))?;
        let path = cursor
            .string()
            .map_err(|message| self.error(line, cursor.column(), message))?;
        cursor
            .end()
            .map_err(|message| self.error(line, cursor.column(), message))?;
        Ok((
            name,
            ImageDef {
                path,
                span: self.span(line.number, 1),
            },
        ))
    }

    fn parse_display_layer(
        &self,
        line: &Line,
        cursor: &mut Cursor<'_>,
    ) -> Result<(String, DisplayLayerDef), Diagnostic> {
        let name = cursor
            .identifier()
            .ok_or_else(|| self.error(line, cursor.column(), "expected display layer name"))?;
        if crate::syntax::builtin_display_layer_order(&name).is_some() {
            return Err(self.error(
                line,
                1,
                format!("display layer `{name}` is built in and cannot be redeclared"),
            ));
        }
        if !cursor.keyword("order") {
            return Err(self.error(line, cursor.column(), "expected `order`"));
        }
        let order = cursor
            .integer()
            .ok_or_else(|| self.error(line, cursor.column(), "expected integer layer order"))?;
        cursor
            .end()
            .map_err(|message| self.error(line, cursor.column(), message))?;
        Ok((
            name,
            DisplayLayerDef {
                order,
                span: self.span(line.number, 1),
            },
        ))
    }

    fn parse_character(
        &self,
        line: &Line,
        cursor: &mut Cursor<'_>,
    ) -> Result<(String, CharacterDef), Diagnostic> {
        let id = cursor
            .identifier()
            .ok_or_else(|| self.error(line, cursor.column(), "expected character id"))?;
        cursor
            .symbol('=')
            .map_err(|message| self.error(line, cursor.column(), message))?;
        if !cursor.keyword("character") {
            return Err(self.error(line, cursor.column(), "expected `character`"));
        }
        let name = cursor
            .string()
            .map_err(|message| self.error(line, cursor.column(), message))?;
        let color = if cursor.keyword("color") {
            cursor
                .string()
                .map_err(|message| self.error(line, cursor.column(), message))?
        } else {
            "#f4f4f5".to_owned()
        };
        let image = if cursor.keyword("image") {
            Some(cursor.identifier().ok_or_else(|| {
                self.error(line, cursor.column(), "expected character image name")
            })?)
        } else {
            None
        };
        cursor
            .end()
            .map_err(|message| self.error(line, cursor.column(), message))?;
        Ok((
            id,
            CharacterDef {
                name,
                color,
                image,
                span: self.span(line.number, 1),
            },
        ))
    }

    fn parse_label_header(
        &self,
        line: &Line,
        cursor: &mut Cursor<'_>,
    ) -> Result<(String, Vec<LabelParameter>), Diagnostic> {
        let name = cursor
            .identifier()
            .ok_or_else(|| self.error(line, cursor.column(), "expected label name"))?;
        let parameters = if cursor.consume_symbol('(') {
            self.parse_parameter_list(line, cursor)?
        } else {
            Vec::new()
        };
        cursor
            .symbol(':')
            .map_err(|message| self.error(line, cursor.column(), message))?;
        cursor
            .end()
            .map_err(|message| self.error(line, cursor.column(), message))?;
        Ok((name, parameters))
    }

    fn parse_parameter_list(
        &self,
        line: &Line,
        cursor: &mut Cursor<'_>,
    ) -> Result<Vec<LabelParameter>, Diagnostic> {
        let mut values = Vec::new();
        let mut saw_default = false;
        if cursor.consume_symbol(')') {
            return Ok(values);
        }
        loop {
            cursor.skip_spaces();
            let parameter_column = cursor.column();
            let name = cursor
                .identifier()
                .ok_or_else(|| self.error(line, cursor.column(), "expected parameter name"))?;
            if values
                .iter()
                .any(|parameter: &LabelParameter| parameter.name == name)
            {
                return Err(self.error(
                    line,
                    cursor.column(),
                    format!("duplicate parameter `{name}`"),
                ));
            }
            let default = if cursor.consume_symbol('=') {
                saw_default = true;
                cursor.skip_spaces();
                let column = cursor.column();
                let source = cursor
                    .argument_source()
                    .map_err(|message| self.error(line, cursor.column(), message))?;
                Some(parse_expression(
                    source,
                    &self.source_name,
                    line.number,
                    column,
                )?)
            } else {
                if saw_default {
                    return Err(self.error(
                        line,
                        cursor.column(),
                        "required parameters must precede parameters with defaults",
                    ));
                }
                None
            };
            values.push(LabelParameter {
                name,
                default,
                span: self.span(line.number, parameter_column),
            });
            if cursor.consume_symbol(')') {
                break;
            }
            cursor
                .symbol(',')
                .map_err(|message| self.error(line, cursor.column(), message))?;
        }
        Ok(values)
    }
}
