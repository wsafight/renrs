use crate::syntax::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Builtin {
    List,
    Record,
    Get,
    Put,
    Push,
    Remove,
    Len,
    Contains,
}

impl Builtin {
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Some(match name {
            "list" => Self::List,
            "record" => Self::Record,
            "get" => Self::Get,
            "put" => Self::Put,
            "push" => Self::Push,
            "remove" => Self::Remove,
            "len" => Self::Len,
            "contains" => Self::Contains,
            _ => return None,
        })
    }
    #[must_use]
    pub const fn accepts(self, count: usize) -> bool {
        match self {
            Self::List => count <= 128,
            Self::Record => count <= 128 && count.is_multiple_of(2),
            Self::Get => count == 2 || count == 3,
            Self::Put => count == 3,
            Self::Push | Self::Remove | Self::Contains => count == 2,
            Self::Len => count == 1,
        }
    }
}

impl Value {
    /// Validates the deterministic data budget for one story variable.
    /// # Errors
    /// Rejects more than 4096 values, 16 collection levels or 1 MiB of text.
    pub fn validate_data(&self) -> Result<(), &'static str> {
        let mut pending = vec![(self, 0)];
        let mut items = 0;
        let mut bytes = 0;
        while let Some((value, depth)) = pending.pop() {
            items += 1;
            if items > 4096 || depth > 16 {
                return Err("data exceeds 4096 values or 16 nesting levels");
            }
            match value {
                Self::String(text) => bytes += text.len(),
                Self::List(values) => pending.extend(values.iter().map(|value| (value, depth + 1))),
                Self::Record(values) => {
                    bytes += values.keys().map(String::len).sum::<usize>();
                    pending.extend(values.values().map(|value| (value, depth + 1)));
                }
                Self::Integer(_) | Self::Boolean(_) => {}
            }
            if bytes > 1024 * 1024 {
                return Err("data text exceeds 1 MiB");
            }
        }
        Ok(())
    }
}
