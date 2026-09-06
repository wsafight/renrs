use std::collections::{BTreeMap, HashSet};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;
mod plurals;
pub use plurals::PluralMessage;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TranslationId(String);

impl TranslationId {
    /// Creates an author-facing translation/stable anchor.
    ///
    /// # Errors
    ///
    /// IDs must be 1-128 ASCII characters and may contain alphanumerics,
    /// `_`, `-`, `.`, `/`, and `:`.
    pub fn new(value: impl Into<String>) -> Result<Self, LocalizationError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 128
            || !value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'/' | b':')
            })
        {
            return Err(LocalizationError::InvalidId(value));
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn generated(value: String) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for TranslationId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationKind {
    Dialogue,
    Menu,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranslationSource {
    pub id: TranslationId,
    pub text: String,
    pub kind: TranslationKind,
    pub speaker: Option<String>,
}

/// A single-language catalog. `fallback` is consulted before progressively
/// less-specific language tags (for example `zh-Hans-CN` -> `zh-Hans` -> `zh`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranslationCatalog {
    pub language: String,
    #[serde(default)]
    pub fallback: Option<String>,
    #[serde(default)]
    pub messages: BTreeMap<TranslationId, String>,
    #[serde(default)]
    pub plurals: BTreeMap<TranslationId, PluralMessage>,
}

impl TranslationCatalog {
    /// Creates and validates a catalog.
    ///
    /// # Errors
    ///
    /// Returns an error when either language tag is malformed.
    pub fn new(
        language: impl Into<String>,
        fallback: Option<String>,
        messages: BTreeMap<TranslationId, String>,
    ) -> Result<Self, LocalizationError> {
        let catalog = Self {
            language: language.into(),
            fallback,
            messages,
            plurals: BTreeMap::new(),
        };
        catalog.validate()?;
        Ok(catalog)
    }

    /// Decodes a UTF-8 JSON catalog.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed JSON or an invalid language tag.
    pub fn from_reader(reader: impl Read) -> Result<Self, LocalizationError> {
        let catalog: Self = serde_json::from_reader(reader)?;
        catalog.validate()?;
        Ok(catalog)
    }

    /// Loads a UTF-8 JSON catalog from disk.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read or decoded.
    pub fn load(path: &Path) -> Result<Self, LocalizationError> {
        Self::from_reader(BufReader::new(File::open(path)?))
    }

    fn validate(&self) -> Result<(), LocalizationError> {
        validate_language(&self.language)?;
        if let Some(fallback) = &self.fallback {
            validate_language(fallback)?;
        }
        for id in self.messages.keys() {
            Self::validate_message_id(id)?;
        }
        for (id, plural) in &self.plurals {
            Self::validate_message_id(id)?;
            plural.validate(&self.language)?;
        }
        Ok(())
    }

    fn validate_message_id(id: &TranslationId) -> Result<(), LocalizationError> {
        TranslationId::new(id.as_str()).map(|_| ())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Localizer {
    language: Option<String>,
    catalogs: BTreeMap<String, TranslationCatalog>,
}

impl Localizer {
    /// Returns the installed language tags in a stable order.
    pub fn languages(&self) -> impl Iterator<Item = &str> {
        self.catalogs.keys().map(String::as_str)
    }

    #[must_use]
    pub fn new(language: Option<String>) -> Self {
        Self {
            language,
            catalogs: BTreeMap::new(),
        }
    }

    /// Changes the active language. `None` selects source text.
    ///
    /// # Errors
    ///
    /// Returns an error for a malformed language tag.
    pub fn set_language(&mut self, language: Option<String>) -> Result<(), LocalizationError> {
        if let Some(value) = &language {
            validate_language(value)?;
        }
        self.language = language;
        Ok(())
    }

    #[must_use]
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    /// Installs or replaces a catalog by its language tag.
    ///
    /// # Errors
    ///
    /// Returns an error when the language tag is malformed.
    /// Installs or replaces a language catalog.
    ///
    /// # Errors
    ///
    /// Returns an error if the catalog contains an invalid language tag.
    pub fn insert(&mut self, catalog: TranslationCatalog) -> Result<(), LocalizationError> {
        catalog.validate()?;
        self.catalogs.insert(catalog.language.clone(), catalog);
        Ok(())
    }

    #[must_use]
    pub fn translate<'a>(&'a self, id: &TranslationId, source: &'a str) -> &'a str {
        self.resolve(id, source, None).unwrap_or(source)
    }

    /// Selects a catalog plural using an integer story variable.
    /// # Errors
    /// Returns an error when the count variable is absent or is not an integer.
    pub fn translate_values<'a>(
        &'a self,
        id: &TranslationId,
        source: &'a str,
        variables: &BTreeMap<String, crate::syntax::Value>,
    ) -> Result<&'a str, LocalizationError> {
        self.resolve(id, source, Some(variables))
    }

    fn resolve<'a>(
        &'a self,
        id: &TranslationId,
        source: &'a str,
        variables: Option<&BTreeMap<String, crate::syntax::Value>>,
    ) -> Result<&'a str, LocalizationError> {
        let Some(language) = &self.language else {
            return Ok(source);
        };
        let mut pending = language_fallbacks(language);
        let mut visited = HashSet::new();
        while let Some(candidate) = pending.first().cloned() {
            pending.remove(0);
            if !visited.insert(candidate.clone()) {
                continue;
            }
            let Some(catalog) = self.catalogs.get(&candidate) else {
                continue;
            };
            if let Some(plural) = catalog.plurals.get(id) {
                return plural.select(&catalog.language, variables);
            }
            if let Some(value) = catalog.messages.get(id).filter(|value| !value.is_empty()) {
                return Ok(value);
            }
            if let Some(fallback) = &catalog.fallback {
                pending.splice(0..0, language_fallbacks(fallback));
            }
        }
        Ok(source)
    }
}

#[derive(Debug, Error)]
pub enum LocalizationError {
    #[error("invalid translation id `{0}`")]
    InvalidId(String),
    #[error("invalid language tag `{0}`")]
    InvalidLanguage(String),
    #[error("invalid plural translation: {0}")]
    InvalidPlural(String),
    #[error("could not read translation catalog: {0}")]
    Io(#[from] std::io::Error),
    #[error("translation catalog is invalid: {0}")]
    Format(#[from] serde_json::Error),
}

fn validate_language(value: &str) -> Result<(), LocalizationError> {
    let valid = !value.is_empty()
        && value.len() <= 64
        && value.split('-').all(|part| {
            !part.is_empty()
                && part.len() <= 8
                && part.bytes().all(|byte| byte.is_ascii_alphanumeric())
        });
    if valid {
        Ok(())
    } else {
        Err(LocalizationError::InvalidLanguage(value.to_owned()))
    }
}

fn language_fallbacks(language: &str) -> Vec<String> {
    let mut parts = language.split('-').collect::<Vec<_>>();
    let mut output = Vec::with_capacity(parts.len());
    while !parts.is_empty() {
        output.push(parts.join("-"));
        parts.pop();
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_explicit_and_structural_language_fallbacks() {
        let id = TranslationId::new("intro.hello").unwrap();
        let mut localizer = Localizer::new(Some("zh-Hans-CN".to_owned()));
        localizer
            .insert(TranslationCatalog {
                language: "zh".to_owned(),
                fallback: None,
                plurals: BTreeMap::new(),
                messages: BTreeMap::from([(id.clone(), "你好".to_owned())]),
            })
            .unwrap();
        assert_eq!(localizer.translate(&id, "Hello"), "你好");
        assert_eq!(
            localizer.translate(&TranslationId::new("missing").unwrap(), "Source"),
            "Source"
        );
    }

    #[test]
    fn rejects_unsafe_ids_and_language_tags() {
        assert!(TranslationId::new("space is unsafe").is_err());
        assert!(
            Localizer::new(None)
                .set_language(Some("zh_中文".to_owned()))
                .is_err()
        );
    }

    #[test]
    fn rejects_invalid_catalog_message_ids() {
        let error = TranslationCatalog::from_reader(
            br#"{"language":"en","messages":{"not valid":"Hello"}}"#.as_slice(),
        )
        .unwrap_err();
        assert!(matches!(error, LocalizationError::InvalidId(_)));
    }
}
