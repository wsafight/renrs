//! Small text predicates and canonical string keys.

/// Whether `character` can start a script identifier.
#[must_use]
pub const fn is_identifier_start(character: char) -> bool {
    character == '_' || character.is_ascii_alphabetic()
}

/// Whether `character` can continue a script identifier.
#[must_use]
pub const fn is_identifier_continue(character: char) -> bool {
    is_identifier_start(character) || character.is_ascii_digit()
}

/// Whether `value` is a non-empty ASCII identifier.
#[must_use]
pub fn is_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    characters.next().is_some_and(is_identifier_start) && characters.all(is_identifier_continue)
}

/// Parses `#rrggbb` or `#rrggbbaa` into RGBA, defaulting alpha to 255.
///
/// Returns `None` for a missing `#`, a wrong length, or non-hex digits.
#[must_use]
pub fn parse_hex_color(input: &str) -> Option<[u8; 4]> {
    let hex = input.strip_prefix('#')?;
    if !matches!(hex.len(), 6 | 8) || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
    let alpha = if hex.len() == 8 {
        u8::from_str_radix(&hex[6..8], 16).ok()?
    } else {
        255
    };
    Some([red, green, blue, alpha])
}

/// Whether `input` is a valid `#rrggbb` or `#rrggbbaa` color.
#[must_use]
pub fn is_hex_color(input: &str) -> bool {
    parse_hex_color(input).is_some()
}

/// Escapes backslashes and double quotes for a quoted literal.
#[must_use]
pub fn escape_quoted(source: &str) -> String {
    source.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Canonical translation key for a built-in UI label.
///
/// Both the native player and the Web build derive UI keys here, so the two
/// surfaces cannot drift into different catalog keys.
#[must_use]
pub fn ui_key(text: &str) -> String {
    format!(
        "ui.{}",
        text.to_ascii_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join("_")
    )
}
