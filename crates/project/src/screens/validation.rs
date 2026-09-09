use super::{SCREENS_FILE, Screens, Widget, layout};

pub(super) fn validate_story_map(screens: &Screens) -> Result<(), String> {
    if screens.story.len() > 32 {
        return Err("at most 32 story screens".to_owned());
    }
    for (name, screen) in &screens.story {
        if name.is_empty() || name.len() > 64 {
            return Err("story screen names must be 1..64 characters".to_owned());
        }
        let elements = layout(screen)?;
        for element in &elements {
            validate_visible(element.visible.as_deref())?;
            if element
                .style
                .as_ref()
                .is_some_and(|style| !screens.styles.contains_key(style))
            {
                return Err("unknown screen style".to_owned());
            }
            match &element.widget {
                Widget::Hotspot {
                    variable,
                    expression,
                    ..
                } => validate_hotspot(variable.as_deref(), expression.as_deref())?,
                Widget::Button { .. } | Widget::Text { .. } | Widget::Image { .. } => {}
                _ => {
                    return Err(
                        "story screens support only hotspot, button, text and image widgets"
                            .to_owned(),
                    );
                }
            }
        }
    }
    Ok(())
}

pub(super) fn validate_visible(source: Option<&str>) -> Result<(), String> {
    source.map_or(Ok(()), |source| {
        renrs_compiler::expression::parse_expression(source, SCREENS_FILE, 1, 1)
            .map(|_| ())
            .map_err(|error| error.to_string())
    })
}

pub(super) fn validate_hotspot(
    variable: Option<&str>,
    expression: Option<&str>,
) -> Result<(), String> {
    match (variable, expression) {
        (None, None) => Ok(()),
        (Some(variable), Some(expression)) => {
            if !matches!(
                renrs_compiler::expression::parse_expression(variable, SCREENS_FILE, 1, 1),
                Ok(renrs_syntax::syntax::Expr::Variable(_))
            ) {
                return Err("hotspot target must be a variable name".to_owned());
            }
            renrs_compiler::expression::parse_expression(expression, SCREENS_FILE, 1, 1)
                .map(|_| ())
                .map_err(|error| error.to_string())
        }
        _ => Err("hotspot variable and expression must be provided together".to_owned()),
    }
}
