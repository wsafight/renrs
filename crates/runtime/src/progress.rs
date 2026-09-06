use crate::syntax::Value;
use crate::{Runtime, RuntimeError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub use renrs_compiler::progress::{PROGRESS_FILE, ProgressConfig, Unlock};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Profile {
    pub project_id: String,
    pub achievements: BTreeSet<String>,
    pub gallery: BTreeSet<String>,
    pub endings: BTreeSet<String>,
    pub variables: BTreeMap<String, Value>,
}

impl Runtime {
    /// Installs cross-playthrough data. Save restoration never replaces this profile.
    /// # Errors
    /// Rejects data belonging to another project or non-persistent variable names.
    pub fn set_profile(&mut self, mut profile: Profile) -> Result<(), RuntimeError> {
        if !profile.project_id.is_empty() && profile.project_id != self.program().project_id {
            return Err(RuntimeError::Execution {
                line: 0,
                message: "profile belongs to another project".to_owned(),
            });
        }
        if profile
            .variables
            .keys()
            .any(|name| !name.starts_with("persistent_"))
        {
            return Err(RuntimeError::Execution {
                line: 0,
                message: "profile variables must start with persistent_".to_owned(),
            });
        }
        profile.project_id.clone_from(&self.program().project_id);
        if self.profile != profile {
            self.profile = profile;
            self.profile_revision = self.profile_revision.wrapping_add(1);
        }
        self.restore_persistent_variables();
        Ok(())
    }

    #[must_use]
    pub const fn profile(&self) -> &Profile {
        &self.profile
    }

    #[must_use]
    pub const fn profile_revision(&self) -> u64 {
        self.profile_revision
    }

    pub(crate) fn set_persistent_variable(&mut self, name: &str, value: &Value) {
        if name.starts_with("persistent_") && self.profile.variables.get(name) != Some(value) {
            self.profile
                .variables
                .insert(name.to_owned(), value.clone());
            self.profile_revision = self.profile_revision.wrapping_add(1);
        }
    }

    pub(crate) fn restore_persistent_variables(&mut self) {
        if self
            .profile
            .variables
            .iter()
            .any(|(key, value)| self.variables.get(key) != Some(value))
        {
            std::sync::Arc::make_mut(&mut self.variables).extend(self.profile.variables.clone());
        }
    }

    pub(crate) fn observe_progress(&mut self) {
        let config = &self.program.progress;
        for (items, unlocked) in [
            (&config.achievements, &mut self.profile.achievements),
            (&config.gallery, &mut self.profile.gallery),
            (&config.endings, &mut self.profile.endings),
        ] {
            for item in items {
                if self.program.labels.get(&item.label) == Some(&self.instruction)
                    && unlocked.insert(item.id.clone())
                {
                    self.profile_revision = self.profile_revision.wrapping_add(1);
                }
            }
        }
        if config
            .rollback_barriers
            .iter()
            .any(|label| self.program.labels.get(label) == Some(&self.instruction))
        {
            self.rollback.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profile_survives_rollback_restore_and_new_playthroughs() {
        let mut program = crate::compile(&crate::parse_script("default persistent_count = 0\nlabel start:\n    \"Before\"\n    set persistent_count = persistent_count + 1\n    jump ending\nlabel ending:\n    \"End\"\n    \"After\"", "test.rns").unwrap()).unwrap();
        program.progress.endings.push(Unlock {
            id: "first".to_owned(),
            title: "First".to_owned(),
            label: "ending".to_owned(),
            image: None,
        });
        program.progress.rollback_barriers.push("ending".to_owned());
        program.progress.validate(&program).unwrap();
        let mut runtime = Runtime::new(program.clone()).unwrap();
        runtime.advance().unwrap();
        let old = runtime.snapshot();
        runtime.continue_story().unwrap();
        assert!(!runtime.can_rollback());
        assert!(runtime.profile().endings.contains("first"));
        runtime.continue_story().unwrap();
        runtime.rollback().unwrap();
        let profile = runtime.profile().clone();
        let mut restored = Runtime::restore(program.clone(), old).unwrap();
        restored.set_profile(profile.clone()).unwrap();
        assert_eq!(restored.variables()["persistent_count"], Value::Integer(1));
        let mut next = Runtime::new(program).unwrap();
        next.set_profile(profile).unwrap();
        assert_eq!(next.variables()["persistent_count"], Value::Integer(1));
    }

    #[test]
    fn profile_revision_changes_only_for_persistent_mutations() {
        let program = crate::compile(&crate::parse_script("default persistent_count = 0\ndefault local = 0\nlabel start:\n    \"Before\"\n    set persistent_count = 1\n    \"After\"", "test.rns").unwrap()).unwrap();
        let mut runtime = Runtime::new(program).unwrap();
        runtime.advance().unwrap();
        let revision = runtime.profile_revision();
        runtime
            .set_screen_variable("local", Value::Integer(2))
            .unwrap();
        assert_eq!(runtime.profile_revision(), revision);
        runtime.continue_story().unwrap();
        assert_ne!(runtime.profile_revision(), revision);
        let revision = runtime.profile_revision();
        runtime
            .set_screen_variable("persistent_count", Value::Integer(1))
            .unwrap();
        runtime.rollback().unwrap();
        assert_eq!(runtime.profile_revision(), revision);
    }
}
