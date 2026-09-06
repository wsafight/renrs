use super::Path;

pub(super) struct Cursor<'a> {
    input: &'a str,
    offset: usize,
}

impl<'a> Cursor<'a> {
    pub(super) const fn new(input: &'a str) -> Self {
        Self { input, offset: 0 }
    }

    pub(super) fn keyword(&mut self, expected: &str) -> bool {
        self.skip_spaces();
        let rest = &self.input[self.offset..];
        if !rest.starts_with(expected) {
            return false;
        }
        let end = self.offset + expected.len();
        if self.input[end..]
            .chars()
            .next()
            .is_some_and(is_identifier_continue)
        {
            return false;
        }
        self.offset = end;
        true
    }

    pub(super) fn identifier(&mut self) -> Option<String> {
        self.skip_spaces();
        let start = self.offset;
        let first = self.peek()?;
        if !is_identifier_start(first) {
            return None;
        }
        self.bump();
        while self.peek().is_some_and(is_identifier_continue) {
            self.bump();
        }
        Some(self.input[start..self.offset].to_owned())
    }

    pub(super) fn consume_symbol(&mut self, expected: char) -> bool {
        self.skip_spaces();
        if self.peek() == Some(expected) {
            self.bump();
            true
        } else {
            false
        }
    }

    pub(super) fn integer(&mut self) -> Option<i32> {
        self.skip_spaces();
        let start = self.offset;
        if self.peek() == Some('-') {
            self.bump();
        }
        let digits = self.offset;
        while self
            .peek()
            .is_some_and(|character| character.is_ascii_digit())
        {
            self.bump();
        }
        if self.offset == digits {
            self.offset = start;
            return None;
        }
        self.input[start..self.offset].parse().ok()
    }

    pub(super) fn number(&mut self) -> Option<f32> {
        self.skip_spaces();
        let start = self.offset;
        while self
            .peek()
            .is_some_and(|character| !character.is_whitespace())
        {
            self.bump();
        }
        if let Ok(value) = self.input[start..self.offset].parse() {
            Some(value)
        } else {
            self.offset = start;
            None
        }
    }

    pub(super) fn string(&mut self) -> Result<String, &'static str> {
        self.skip_spaces();
        if self.bump() != Some('"') {
            return Err("expected quoted string");
        }
        let mut output = String::new();
        while let Some(ch) = self.bump() {
            match ch {
                '"' => return Ok(output),
                '\\' => match self.bump() {
                    Some('n') => output.push('\n'),
                    Some('r') => output.push('\r'),
                    Some('t') => output.push('\t'),
                    Some('"') => output.push('"'),
                    Some('\\') => output.push('\\'),
                    Some(_) => return Err("unsupported string escape"),
                    None => return Err("unterminated string literal"),
                },
                _ => output.push(ch),
            }
        }
        Err("unterminated string literal")
    }

    pub(super) fn symbol(&mut self, expected: char) -> Result<(), String> {
        self.skip_spaces();
        if self.bump() == Some(expected) {
            Ok(())
        } else {
            Err(format!("expected `{expected}`"))
        }
    }

    pub(super) fn rest(&mut self) -> &'a str {
        self.skip_spaces();
        let output = self.input[self.offset..].trim_end();
        self.offset = self.input.len();
        output
    }

    pub(super) fn rest_before_colon(&mut self) -> Result<&'a str, &'static str> {
        self.skip_spaces();
        let rest = self.input[self.offset..].trim_end();
        let Some(expression) = rest.strip_suffix(':') else {
            return Err("expected `:` after condition");
        };
        let expression = expression.trim_end();
        if expression.is_empty() {
            return Err("expected condition before `:`");
        }
        self.offset = self.input.len();
        Ok(expression)
    }

    pub(super) fn argument_source(&mut self) -> Result<&'a str, &'static str> {
        self.skip_spaces();
        let start = self.offset;
        let mut depth = 0_usize;
        let mut quoted = false;
        let mut escaped = false;
        while let Some(ch) = self.peek() {
            if escaped {
                escaped = false;
                self.bump();
                continue;
            }
            match ch {
                '\\' if quoted => escaped = true,
                '"' => quoted = !quoted,
                '(' if !quoted => depth += 1,
                ')' | ',' if !quoted && depth == 0 => break,
                ')' if !quoted => depth -= 1,
                _ => {}
            }
            self.bump();
        }
        if quoted || depth != 0 {
            return Err("unterminated call argument");
        }
        let source = self.input[start..self.offset].trim();
        if source.is_empty() {
            Err("expected call argument")
        } else {
            Ok(source)
        }
    }

    pub(super) fn end(&mut self) -> Result<(), &'static str> {
        self.skip_spaces();
        if self.offset == self.input.len() {
            Ok(())
        } else {
            Err("unexpected text at end of statement")
        }
    }

    pub(super) const fn column(&self) -> usize {
        self.offset + 1
    }

    pub(super) fn skip_spaces(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.bump();
        }
    }

    pub(super) fn peek(&self) -> Option<char> {
        self.input[self.offset..].chars().next()
    }

    pub(super) fn peek_non_space(&mut self) -> Option<char> {
        self.skip_spaces();
        self.peek()
    }

    pub(super) fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.offset += ch.len_utf8();
        Some(ch)
    }
}

pub(super) fn starts_keyword(input: &str, keyword: &str) -> bool {
    let mut cursor = Cursor::new(input);
    cursor.keyword(keyword)
}

pub(super) fn default_alias(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("image")
        .chars()
        .map(|ch| if is_identifier_continue(ch) { ch } else { '_' })
        .collect()
}

pub(crate) fn valid_project_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        && value.bytes().any(|byte| byte.is_ascii_alphanumeric())
}

#[must_use]
pub fn derive_project_id(title: &str) -> String {
    let mut id = title
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    while id.contains("--") {
        id = id.replace("--", "-");
    }
    let id = id.trim_matches('-');
    if id.is_empty() {
        "renrs-game".to_owned()
    } else {
        id.chars().take(128).collect()
    }
}

const fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

const fn is_identifier_continue(ch: char) -> bool {
    is_identifier_start(ch) || ch.is_ascii_digit()
}
