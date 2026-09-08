use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use super::expressions::valid_identifier;

#[derive(Debug, Clone)]
pub(super) struct StaticTransform {
    pub(super) position: Option<&'static str>,
    steps: Vec<TransformStep>,
    assumed_easing: bool,
    assumed_pause: bool,
}

#[derive(Debug, Clone, Default)]
struct TransformStep {
    properties: Vec<(&'static str, f32)>,
    seconds: f32,
    easing: &'static str,
    pause: bool,
}

#[derive(Debug, Default)]
pub(super) struct TransformCatalog {
    transforms: BTreeMap<String, StaticTransform>,
    declaration_lines: BTreeSet<usize>,
}

impl TransformCatalog {
    pub(super) fn parse(source: &str) -> Self {
        let lines = source.lines().collect::<Vec<_>>();
        let mut catalog = Self::default();
        let mut index = 0;
        while index < lines.len() {
            let raw = lines[index].trim_start_matches('\u{feff}');
            let indent = indentation(raw);
            let content = raw.trim();
            let Some(name) = transform_name(content).filter(|_| indent == 0) else {
                index += 1;
                continue;
            };
            let start = index;
            index += 1;
            let mut body = Vec::new();
            while index < lines.len() {
                let next = lines[index].trim_start_matches('\u{feff}');
                let next_content = next.trim();
                if !next_content.is_empty() && indentation(next) <= indent {
                    break;
                }
                if !next_content.is_empty() && !next_content.starts_with('#') {
                    body.push(next_content);
                }
                index += 1;
            }
            if let Some(transform) = parse_body(&body) {
                catalog.transforms.insert(name.to_owned(), transform);
                catalog.declaration_lines.insert(start + 1);
            }
        }
        catalog
    }

    pub(super) fn recognized_declaration(&self, line: usize) -> bool {
        self.declaration_lines.contains(&line)
    }

    pub(super) fn get(&self, name: &str) -> Option<&StaticTransform> {
        self.transforms.get(name)
    }
}

impl StaticTransform {
    pub(super) fn statements(&self, alias: &str) -> Vec<String> {
        self.steps
            .iter()
            .filter(|step| step.pause || !step.properties.is_empty())
            .map(|step| {
                let mut statement = format!("transform {alias}");
                if step.pause {
                    return format!("pause {}", step.seconds);
                }
                for (name, value) in &step.properties {
                    write!(statement, " {name} {value}").expect("writing to a String cannot fail");
                }
                if step.seconds > 0.0 {
                    write!(statement, " over {} ease {}", step.seconds, step.easing)
                        .expect("writing to a String cannot fail");
                }
                statement
            })
            .collect()
    }

    pub(super) fn assumption(&self) -> Option<String> {
        match (self.assumed_easing, self.assumed_pause) {
            (true, true) => Some(
                "mapped Ren'Py `ease` to `ease in_out` and ATL pause to a blocking RenRS pause"
                    .to_owned(),
            ),
            (true, false) => {
                Some("mapped Ren'Py `ease` interpolation to RenRS `ease in_out`".to_owned())
            }
            (false, true) => Some("mapped Ren'Py ATL pause to a blocking RenRS pause".to_owned()),
            (false, false) => None,
        }
    }

    pub(super) fn camera_statements(&self) -> Option<Vec<String>> {
        self.position.is_none().then(|| self.statements("camera"))
    }
}

fn parse_body(lines: &[&str]) -> Option<StaticTransform> {
    if lines.is_empty() {
        return None;
    }
    let mut transform = StaticTransform {
        position: None,
        steps: vec![TransformStep {
            easing: "linear",
            ..TransformStep::default()
        }],
        assumed_easing: false,
        assumed_pause: false,
    };
    for line in lines {
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        if let ["pause", seconds] = tokens.as_slice() {
            transform.assumed_pause = true;
            transform.steps.push(TransformStep {
                seconds: parse_number(seconds, false)?,
                pause: true,
                ..TransformStep::default()
            });
            continue;
        }
        let (offset, seconds, easing) = match tokens.as_slice() {
            ["linear", seconds, ..] => (2, parse_number(seconds, false)?, "linear"),
            ["easein", seconds, ..] => (2, parse_number(seconds, false)?, "in"),
            ["easeout", seconds, ..] => (2, parse_number(seconds, false)?, "out"),
            ["ease", seconds, ..] => {
                transform.assumed_easing = true;
                (2, parse_number(seconds, false)?, "in_out")
            }
            _ => (0, 0.0, "linear"),
        };
        let properties = parse_properties(&tokens[offset..], &mut transform.position)?;
        if offset == 0 {
            transform.steps[0].properties.extend(properties);
        } else {
            transform.steps.push(TransformStep {
                properties,
                seconds,
                easing,
                pause: false,
            });
        }
    }
    Some(transform)
}

fn parse_properties(
    tokens: &[&str],
    position: &mut Option<&'static str>,
) -> Option<Vec<(&'static str, f32)>> {
    let mut result = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let property = tokens[index];
        let value = *tokens.get(index + 1)?;
        match property {
            "xalign" => {
                *position = Some(match parse_number(value, true)? {
                    value if value.abs() < f32::EPSILON => "left",
                    value if (value - 0.5).abs() < f32::EPSILON => "center",
                    value if (value - 1.0).abs() < f32::EPSILON => "right",
                    _ => return None,
                });
            }
            "yalign" if (parse_number(value, true)? - 1.0).abs() < f32::EPSILON => {}
            "alpha" => result.push(("alpha", parse_number(value, true)?)),
            "zoom" => {
                let value = parse_number(value, false)?;
                if !(0.01..=20.0).contains(&value) {
                    return None;
                }
                result.push(("scale", value));
            }
            "rotate" => result.push(("rotate", parse_number(value, false)?)),
            "xoffset" => result.push(("x", parse_number(value, false)?)),
            "yoffset" => result.push(("y", parse_number(value, false)?)),
            _ => return None,
        }
        index += 2;
    }
    Some(result)
}

fn parse_number(value: &str, unit: bool) -> Option<f32> {
    let value = value.parse::<f32>().ok()?;
    (value.is_finite() && (!unit || (0.0..=1.0).contains(&value))).then_some(value)
}

fn transform_name(source: &str) -> Option<&str> {
    let name = source.strip_prefix("transform ")?.strip_suffix(':')?.trim();
    valid_identifier(name).then_some(name)
}

fn indentation(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_static_position_and_animation() {
        let catalog = TransformCatalog::parse(
            "transform reveal:\n    xalign 0.0\n    yalign 1.0\n    alpha 0.0\n    linear 0.5 alpha 1.0 xoffset 20\n",
        );
        let transform = catalog.get("reveal").unwrap();
        assert_eq!(transform.position, Some("left"));
        assert_eq!(
            transform.statements("hero"),
            [
                "transform hero alpha 0",
                "transform hero alpha 1 x 20 over 0.5 ease linear"
            ]
        );
    }

    #[test]
    fn rejects_non_equivalent_alignment() {
        assert!(
            TransformCatalog::parse("transform quarter:\n    xalign 0.25\n")
                .get("quarter")
                .is_none()
        );
    }

    #[test]
    fn maps_directional_easing_and_static_pauses() {
        let catalog = TransformCatalog::parse(
            "transform reveal:\n    alpha 0\n    easein 0.2 alpha 1\n    pause 0.4\n    easeout 0.3 alpha 0\n",
        );
        assert_eq!(
            catalog.get("reveal").unwrap().statements("hero"),
            [
                "transform hero alpha 0",
                "transform hero alpha 1 over 0.2 ease in",
                "pause 0.4",
                "transform hero alpha 0 over 0.3 ease out",
            ]
        );
    }
}
