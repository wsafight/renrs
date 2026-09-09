use std::fmt::Write;

use super::conversion::{LineConversion, unsupported};
use super::expressions::escape_string;

pub(super) fn convert_audio(content: &str) -> Option<LineConversion> {
    if let Some(rest) = content.strip_prefix("play music ") {
        return Some(convert_music("play music", rest, true));
    }
    if let Some(rest) = content.strip_prefix("queue music ") {
        return Some(convert_music("queue music", rest, true));
    }
    if let Some(rest) = content.strip_prefix("play sound ") {
        return Some(convert_sound("play sound", rest));
    }
    if let Some(rest) = content.strip_prefix("queue sound ") {
        return Some(convert_sound("queue sound", rest));
    }
    if let Some(rest) = content.strip_prefix("voice ") {
        let (path, options) = static_path(rest)?;
        return Some(if options.is_empty() {
            LineConversion::One(format!("voice \"{}\"", escape_string(&path)))
        } else {
            unsupported("voice clauses require manual migration", false)
        });
    }
    if let Some(rest) = content.strip_prefix("stop music") {
        return Some(convert_stop("stop music", rest));
    }
    if let Some(rest) = content.strip_prefix("stop sound") {
        return Some(convert_stop("stop sound", rest));
    }
    if let Some(rest) = content.strip_prefix("stop voice") {
        return Some(convert_stop("stop voice", rest));
    }
    None
}

fn convert_stop(command: &str, rest: &str) -> LineConversion {
    let rest = rest.trim();
    if rest.is_empty() {
        LineConversion::One(command.to_owned())
    } else if let Some(duration) = option_number(rest, "fadeout") {
        LineConversion::One(format!("{command} fadeout {duration}"))
    } else {
        unsupported("only static fadeout is supported", false)
    }
}

fn convert_music(command: &str, source: &str, default_loop: bool) -> LineConversion {
    let Some((path, options)) = static_path(source) else {
        return unsupported("music path must be a static string", false);
    };
    let mut repeat = default_loop;
    let mut if_changed = false;
    let (mut fade_in, mut volume) = (None, None);
    let tokens = options.split_whitespace().collect::<Vec<_>>();
    let mut index = 0;
    while index < tokens.len() {
        match tokens[index] {
            "loop" => {
                repeat = true;
                index += 1;
            }
            "noloop" => {
                repeat = false;
                index += 1;
            }
            "fadein" if fade_in.is_none() => {
                let Some(value) = number(tokens.get(index + 1).copied(), false) else {
                    return unsupported("music fadein must be a static non-negative number", false);
                };
                fade_in = Some(value);
                index += 2;
            }
            "volume" if volume.is_none() => {
                let Some(value) = number(tokens.get(index + 1).copied(), true) else {
                    return unsupported(
                        "music volume must be a static number between 0 and 1",
                        false,
                    );
                };
                volume = Some(value);
                index += 2;
            }
            "if_changed" => {
                if_changed = true;
                index += 1;
            }
            "fadeout" => {
                return unsupported(
                    "music replacement fadeout requires manual sequencing before playback",
                    false,
                );
            }
            _ => return unsupported("music clause is outside the supported static subset", false),
        }
    }
    let mut result = format!("{command} \"{}\"", escape_string(&path));
    if let Some(value) = fade_in {
        write!(result, " fadein {value}").expect("writing to a String cannot fail");
    }
    if let Some(value) = volume {
        write!(result, " volume {value}").expect("writing to a String cannot fail");
    }
    if repeat {
        result.push_str(" loop");
    }
    if if_changed {
        result.push_str(" if_changed");
    }
    LineConversion::One(result)
}

fn convert_sound(command: &str, source: &str) -> LineConversion {
    let Some((path, options)) = static_path(source) else {
        return unsupported("sound path must be a static string", false);
    };
    let mut repeat = false;
    let mut volume = None;
    let tokens = options.split_whitespace().collect::<Vec<_>>();
    let mut index = 0;
    while index < tokens.len() {
        match tokens[index] {
            "loop" => {
                repeat = true;
                index += 1;
            }
            "volume" => {
                let Some(value) = number(tokens.get(index + 1).copied(), true) else {
                    return unsupported(
                        "sound volume must be a static number between 0 and 1",
                        false,
                    );
                };
                volume = Some(value);
                index += 2;
            }
            _ => return unsupported("only static sound volume and loop are supported", false),
        }
    }
    let mut result = format!("{command} \"{}\"", escape_string(&path));
    if let Some(volume) = volume {
        write!(result, " volume {volume}").expect("writing to a String cannot fail");
    }
    if repeat {
        result.push_str(" loop");
    }
    LineConversion::One(result)
}

fn static_path(source: &str) -> Option<(String, &str)> {
    let source = source.trim();
    let quote = source.chars().next()?;
    if !matches!(quote, '\'' | '"') {
        return None;
    }
    let mut escaped = false;
    let mut end = None;
    for (index, character) in source.char_indices().skip(1) {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == quote {
            end = Some(index + character.len_utf8());
            break;
        }
    }
    let end = end?;
    let path = unescape(&source[quote.len_utf8()..end - quote.len_utf8()]);
    Some((path, source[end..].trim()))
}

fn unescape(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let mut escaped = false;
    for character in source.chars() {
        if escaped {
            result.push(match character {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                other => other,
            });
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            result.push(character);
        }
    }
    if escaped {
        result.push('\\');
    }
    result
}

fn option_number(source: &str, option: &str) -> Option<f32> {
    let mut tokens = source.split_whitespace();
    if tokens.next()? != option {
        return None;
    }
    let value = number(tokens.next(), option == "volume")?;
    tokens.next().is_none().then_some(value)
}

fn number(source: Option<&str>, unit: bool) -> Option<f32> {
    let value = source?.parse::<f32>().ok()?;
    (value.is_finite() && value >= 0.0 && (!unit || value <= 1.0)).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_music_defaults_and_static_clauses() {
        assert!(matches!(
            convert_audio("play music \"theme.ogg\"").unwrap(),
            LineConversion::One(ref value) if value.ends_with(" loop")
        ));
        assert!(matches!(
            convert_audio("queue music 'next.ogg' noloop fadein .2 volume .7").unwrap(),
            LineConversion::One(ref value)
                if value == "queue music \"next.ogg\" fadein 0.2 volume 0.7"
        ));
        assert!(matches!(
            convert_audio("stop music fadeout 1.0").unwrap(),
            LineConversion::One(ref value) if value == "stop music fadeout 1"
        ));
    }
}
