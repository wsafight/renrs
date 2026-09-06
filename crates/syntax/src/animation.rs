use crate::syntax::{Easing, TransformProperties};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnimationStep {
    Transform {
        alias: String,
        properties: TransformProperties,
        seconds: f32,
        easing: Easing,
    },
    Pause {
        seconds: f32,
    },
}

impl AnimationStep {
    #[must_use]
    pub const fn seconds(&self) -> f32 {
        match self {
            Self::Transform { seconds, .. } | Self::Pause { seconds } => *seconds,
        }
    }
}

/// Checks independent tracks and returns their total duration.
/// # Errors
/// Rejects conflicting aliases, unbounded tracks or invalid durations.
pub fn validate_tracks(tracks: &[Vec<AnimationStep>]) -> Result<f32, String> {
    if !(2..=16).contains(&tracks.len()) || tracks.iter().map(Vec::len).sum::<usize>() > 256 {
        return Err("parallel requires 2..16 tracks and at most 256 keyframes".to_owned());
    }
    let mut owners = BTreeMap::new();
    let mut duration = 0.0_f32;
    for (index, track) in tracks.iter().enumerate() {
        if track.is_empty() {
            return Err("parallel tracks cannot be empty".to_owned());
        }
        let mut aliases = BTreeSet::new();
        let mut seconds = 0.0;
        for step in track {
            if !step.seconds().is_finite() || step.seconds() < 0.0 {
                return Err("parallel duration must be finite and nonnegative".to_owned());
            }
            seconds += step.seconds();
            if let AnimationStep::Transform { alias, .. } = step {
                aliases.insert(alias);
            }
        }
        for alias in aliases {
            if owners.insert(alias, index).is_some() {
                return Err(format!("parallel tracks both modify `{alias}`"));
            }
        }
        if !seconds.is_finite() || seconds > 86400.0 {
            return Err("parallel track exceeds one day".to_owned());
        }
        duration = duration.max(seconds);
    }
    Ok(duration)
}
