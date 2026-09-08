use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::CompiledProgram;

pub const PROGRESS_FILE: &str = "progress.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProgressConfig {
    pub achievements: Vec<Unlock>,
    pub gallery: Vec<Unlock>,
    pub endings: Vec<Unlock>,
    pub rollback_barriers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unlock {
    pub id: String,
    pub title: String,
    pub label: String,
    #[serde(default)]
    pub image: Option<String>,
}

impl ProgressConfig {
    /// Validates collection identities and label references.
    /// # Errors
    /// Rejects duplicate IDs, absent labels and unsafe gallery paths.
    pub fn validate(&self, program: &CompiledProgram) -> Result<(), String> {
        for items in [&self.achievements, &self.gallery, &self.endings] {
            let mut ids = BTreeSet::new();
            for item in items {
                if item.id.is_empty() || !ids.insert(&item.id) {
                    return Err(format!("duplicate or empty collection ID {}", item.id));
                }
                if !program.labels.contains_key(&item.label) {
                    return Err(format!("unknown unlock label {}", item.label));
                }
                if item.image.as_ref().is_some_and(|path| {
                    path.starts_with('/')
                        || path.contains(['\\', ':'])
                        || path
                            .split('/')
                            .any(|part| part.is_empty() || part == ".." || part == ".")
                }) {
                    return Err("unsafe gallery image path".to_owned());
                }
            }
        }
        for label in &self.rollback_barriers {
            if !program.labels.contains_key(label) {
                return Err(format!("unknown rollback barrier label {label}"));
            }
        }
        Ok(())
    }
}
