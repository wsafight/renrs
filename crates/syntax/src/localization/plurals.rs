use super::LocalizationError;
use crate::syntax::Value;
use intl_pluralrules::{PluralCategory, PluralRuleType, PluralRules};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluralMessage {
    pub count: String,
    pub forms: BTreeMap<String, String>,
}

fn rules(language: &str) -> Result<PluralRules, LocalizationError> {
    super::language_fallbacks(language)
        .into_iter()
        .find_map(|tag| {
            tag.parse::<unic_langid::LanguageIdentifier>()
                .ok()
                .and_then(|tag| PluralRules::create(tag, PluralRuleType::CARDINAL).ok())
        })
        .ok_or_else(|| {
            LocalizationError::InvalidPlural(format!("unsupported plural language {language}"))
        })
}

impl PluralMessage {
    pub(super) fn validate(&self, language: &str) -> Result<(), LocalizationError> {
        rules(language)?;
        if self.count.is_empty()
            || !self.count.bytes().enumerate().all(|(i, ch)| {
                ch == b'_' || ch.is_ascii_alphabetic() || i > 0 && ch.is_ascii_digit()
            })
            || self.forms.get("other").is_none_or(String::is_empty)
            || self.forms.iter().any(|(key, value)| {
                !matches!(
                    key.as_str(),
                    "zero" | "one" | "two" | "few" | "many" | "other"
                ) || value.is_empty()
            })
        {
            return Err(LocalizationError::InvalidPlural(
                "expected count variable, valid forms and non-empty other form".to_owned(),
            ));
        }
        Ok(())
    }

    pub(super) fn select(
        &self,
        language: &str,
        variables: Option<&BTreeMap<String, Value>>,
    ) -> Result<&str, LocalizationError> {
        let Some(variables) = variables else {
            return Ok(&self.forms["other"]);
        };
        let Some(Value::Integer(count)) = variables.get(&self.count) else {
            return Err(LocalizationError::InvalidPlural(format!(
                "{} must be an integer",
                self.count
            )));
        };
        let category = rules(language)?
            .select(count.unsigned_abs().to_string().as_str())
            .map_err(|error| LocalizationError::InvalidPlural(error.to_owned()))?;
        let key = match category {
            PluralCategory::ZERO => "zero",
            PluralCategory::ONE => "one",
            PluralCategory::TWO => "two",
            PluralCategory::FEW => "few",
            PluralCategory::MANY => "many",
            PluralCategory::OTHER => "other",
        };
        Ok(self.forms.get(key).unwrap_or(&self.forms["other"]))
    }
}
