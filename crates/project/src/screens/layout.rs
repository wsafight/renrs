use super::{Element, Screen, Widget};
use crate::theme::ThemeRect;
use std::collections::BTreeSet;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ViewportFrame {
    pub id: String,
    pub bounds: ThemeRect,
    pub content_height: f32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PlacedElement {
    pub key: usize,
    pub bounds: ThemeRect,
    pub style: Option<String>,
    pub widget: Widget,
    pub viewports: Vec<ViewportFrame>,
}

#[derive(Default)]
struct Layout {
    output: Vec<PlacedElement>,
    count: usize,
    viewport_ids: BTreeSet<String>,
}

/// Lays out composable containers in the logical canvas.
/// # Errors
/// Rejects excessive nesting, invalid dimensions, duplicate IDs and overflowing content.
pub fn layout(screen: &Screen) -> Result<Vec<PlacedElement>, String> {
    validate_bounds(
        screen.bounds,
        ThemeRect {
            x: 0.0,
            y: 0.0,
            width: 1280.0,
            height: 720.0,
        },
    )?;
    let mut layout = Layout::default();
    layout.place(&screen.root, screen.bounds, 0, None, &[])?;
    Ok(layout.output)
}

impl Layout {
    #[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
    fn place(
        &mut self,
        element: &Element,
        parent: ThemeRect,
        depth: usize,
        inherited: Option<&str>,
        viewports: &[ViewportFrame],
    ) -> Result<(), String> {
        self.count += 1;
        if depth > 16 || self.count > 256 {
            return Err("screen exceeds 16 levels or 256 elements".to_owned());
        }
        let bounds = element.bounds.map_or(parent, |rect| ThemeRect {
            x: parent.x + rect.x,
            y: parent.y + rect.y,
            ..rect
        });
        validate_bounds(bounds, parent)?;
        let style = element.style.as_deref().or(inherited);
        match &element.widget {
            Widget::Viewport {
                id,
                content_height,
                child,
            } => {
                if id.is_empty()
                    || !self.viewport_ids.insert(id.clone())
                    || !content_height.is_finite()
                    || *content_height < bounds.height
                    || *content_height > 65536.0
                {
                    return Err("viewport needs a unique ID and content_height between its height and 65536".to_owned());
                }
                let mut nested = viewports.to_vec();
                nested.push(ViewportFrame {
                    id: id.clone(),
                    bounds,
                    content_height: *content_height,
                });
                self.place(
                    child,
                    ThemeRect {
                        height: *content_height,
                        ..bounds
                    },
                    depth + 1,
                    style,
                    &nested,
                )?;
            }
            Widget::Stack { children } => {
                validate_children(children)?;
                for child in children {
                    self.place(child, bounds, depth + 1, style, viewports)?;
                }
            }
            Widget::Row {
                gap,
                padding,
                children,
            }
            | Widget::Column {
                gap,
                padding,
                children,
            } => {
                validate_children(children)?;
                if !gap.is_finite() || !padding.is_finite() || *gap < 0.0 || *padding < 0.0 {
                    return Err("invalid screen container spacing".to_owned());
                }
                let horizontal = matches!(element.widget, Widget::Row { .. });
                let available = if horizontal {
                    bounds.width
                } else {
                    bounds.height
                } - 2.0 * padding
                    - gap * (children.len() - 1) as f32;
                let fixed: f32 = children.iter().filter_map(|child| child.size).sum();
                if children
                    .iter()
                    .filter_map(|child| child.size)
                    .any(|size| !size.is_finite() || size < 1.0)
                    || fixed > available
                {
                    return Err("fixed screen children exceed available space".to_owned());
                }
                let flexible = children.iter().filter(|child| child.size.is_none()).count();
                let share = (available - fixed) / flexible.max(1) as f32;
                let mut position = if horizontal { bounds.x } else { bounds.y } + padding;
                for child in children {
                    let size = child.size.unwrap_or(share);
                    let rect = if horizontal {
                        ThemeRect {
                            x: position,
                            y: bounds.y + padding,
                            width: size,
                            height: bounds.height - 2.0 * padding,
                        }
                    } else {
                        ThemeRect {
                            x: bounds.x + padding,
                            y: position,
                            width: bounds.width - 2.0 * padding,
                            height: size,
                        }
                    };
                    self.place(child, rect, depth + 1, style, viewports)?;
                    position += size + gap;
                }
            }
            widget => {
                validate_widget(widget, bounds)?;
                self.output.push(PlacedElement {
                    key: self.count,
                    bounds,
                    style: style.map(str::to_owned),
                    widget: widget.clone(),
                    viewports: viewports.to_vec(),
                });
            }
        }
        Ok(())
    }
}

fn validate_children(children: &[Element]) -> Result<(), String> {
    if children.is_empty() || children.len() > 64 {
        Err("containers need 1..64 children".to_owned())
    } else {
        Ok(())
    }
}

fn validate_bounds(bounds: ThemeRect, parent: ThemeRect) -> Result<(), String> {
    if ![bounds.x, bounds.y, bounds.width, bounds.height]
        .iter()
        .all(|value| value.is_finite())
        || bounds.x < parent.x
        || bounds.y < parent.y
        || bounds.width < 1.0
        || bounds.height < 1.0
        || bounds.x + bounds.width > parent.x + parent.width + 0.01
        || bounds.y + bounds.height > parent.y + parent.height + 0.01
    {
        Err("screen element exceeds its parent bounds".to_owned())
    } else {
        Ok(())
    }
}

fn validate_widget(widget: &Widget, bounds: ThemeRect) -> Result<(), String> {
    let message = match widget {
        Widget::Input {
            max_length,
            variable,
            ..
        } if !(1..=1024).contains(max_length)
            || variable.is_empty()
            || !variable
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_') =>
        {
            Some("invalid input variable or length")
        }
        Widget::List {
            item_height, gap, ..
        } if !gap.is_finite() || *gap < 0.0 || !valid_row(*item_height, bounds) => {
            Some("list rows must fit their viewport")
        }
        Widget::DataList { item_height, .. } if !valid_row(*item_height, bounds) => {
            Some("data rows must fit their viewport")
        }
        Widget::Slider { .. } if bounds.height < 64.0 || bounds.width < 200.0 => {
            Some("sliders need at least 200x64 pixels")
        }
        Widget::Dialogue if bounds.height < 140.0 || bounds.width < 240.0 => {
            Some("dialogue needs at least 240x140 pixels")
        }
        Widget::Choices if bounds.height < 64.0 || bounds.width < 200.0 => {
            Some("choices need at least 200x64 pixels")
        }
        _ => None,
    };
    message.map_or(Ok(()), |message| Err(message.to_owned()))
}

fn valid_row(height: f32, bounds: ThemeRect) -> bool {
    height.is_finite() && height >= 24.0 && height <= bounds.height
}

#[cfg(test)]
mod tests;
