use super::expressions::valid_identifier;

pub(super) fn static_invocation(source: &str) -> Result<(&str, Option<&str>), &'static str> {
    let bytes = source.as_bytes();
    let mut end = 0_usize;
    while bytes
        .get(end)
        .is_some_and(|byte| *byte == b'_' || byte.is_ascii_alphanumeric())
    {
        end += 1;
    }
    let name = &source[..end];
    if !valid_identifier(name) {
        return Err("label target must be a static identifier");
    }
    let rest = source[end..].trim();
    if rest.is_empty() {
        return Ok((name, None));
    }
    let Some(arguments) = rest
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
    else {
        return Err("dynamic labels and call clauses require manual migration");
    };
    Ok((name, Some(arguments)))
}

pub(super) fn split_top_level(source: &str) -> Result<Vec<&str>, &'static str> {
    if source.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut parts = Vec::new();
    let mut start = 0_usize;
    let (mut parentheses, mut brackets, mut braces) = (0_usize, 0_usize, 0_usize);
    let (mut quote, mut escaped) = (None, false);
    for (index, character) in source.char_indices() {
        if let Some(current_quote) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == current_quote {
                quote = None;
            }
            continue;
        }
        match character {
            '\'' | '"' => quote = Some(character),
            '(' => parentheses += 1,
            ')' if parentheses > 0 => parentheses -= 1,
            '[' => brackets += 1,
            ']' if brackets > 0 => brackets -= 1,
            '{' => braces += 1,
            '}' if braces > 0 => braces -= 1,
            ',' if parentheses == 0 && brackets == 0 && braces == 0 => {
                let part = source[start..index].trim();
                if part.is_empty() {
                    return Err("parameter or argument list contains an empty item");
                }
                parts.push(part);
                start = index + 1;
            }
            ')' | ']' | '}' => return Err("parameter or argument list is unbalanced"),
            _ => {}
        }
    }
    if quote.is_some() || parentheses != 0 || brackets != 0 || braces != 0 {
        return Err("parameter or argument list is unterminated");
    }
    let last = source[start..].trim();
    if last.is_empty() {
        return Err("parameter or argument list contains an empty item");
    }
    parts.push(last);
    Ok(parts)
}

pub(super) fn top_level_assignment(source: &str) -> Option<(&str, &str)> {
    let bytes = source.as_bytes();
    let (mut parentheses, mut brackets, mut braces) = (0_usize, 0_usize, 0_usize);
    let (mut quote, mut escaped) = (None, false);
    for (index, byte) in bytes.iter().copied().enumerate() {
        if let Some(current_quote) = quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == current_quote {
                quote = None;
            }
            continue;
        }
        match byte {
            b'\'' | b'"' => quote = Some(byte),
            b'(' => parentheses += 1,
            b')' => parentheses = parentheses.saturating_sub(1),
            b'[' => brackets += 1,
            b']' => brackets = brackets.saturating_sub(1),
            b'{' => braces += 1,
            b'}' => braces = braces.saturating_sub(1),
            b'=' if parentheses == 0
                && brackets == 0
                && braces == 0
                && index.checked_sub(1).and_then(|before| bytes.get(before)) != Some(&b'=')
                && index.checked_sub(1).and_then(|before| bytes.get(before)) != Some(&b'!')
                && index.checked_sub(1).and_then(|before| bytes.get(before)) != Some(&b'<')
                && index.checked_sub(1).and_then(|before| bytes.get(before)) != Some(&b'>')
                && bytes.get(index + 1) != Some(&b'=') =>
            {
                return Some((&source[..index], &source[index + 1..]));
            }
            _ => {}
        }
    }
    None
}
