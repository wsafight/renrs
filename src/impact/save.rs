use crate::Program;
use crate::compiler::InstructionKind;

use super::{SaveCompatibility, StableAlias, State, TypeChange};

pub(super) fn save_compatibility(baseline: &State, candidate: &State) -> SaveCompatibility {
    let defaults_added = candidate
        .defaults
        .keys()
        .filter(|name| !baseline.defaults.contains_key(*name))
        .cloned()
        .collect::<Vec<_>>();
    let defaults_removed = baseline
        .defaults
        .keys()
        .filter(|name| !candidate.defaults.contains_key(*name))
        .cloned()
        .collect::<Vec<_>>();
    let default_types_changed = baseline
        .defaults
        .iter()
        .filter_map(|(name, old)| {
            candidate
                .defaults
                .get(name)
                .filter(|new| *new != old)
                .map(|new| TypeChange {
                    name: name.clone(),
                    from: old.clone(),
                    to: new.clone(),
                })
        })
        .collect::<Vec<_>>();
    let (stable_positions_removed, stable_positions_aliased) =
        stable_position_changes(&baseline.program, &candidate.program);
    let changed = baseline.program.fingerprint != candidate.program.fingerprint;
    let unstable_interaction_positions = if changed {
        unstable_count(&baseline.program, |kind| {
            matches!(
                kind,
                InstructionKind::Dialogue { .. } | InstructionKind::Choice { .. }
            )
        })
    } else {
        0
    };
    let unstable_call_sites = if changed {
        unstable_count(&baseline.program, |kind| {
            matches!(kind, InstructionKind::Call { .. })
        })
    } else {
        0
    };
    let mut reasons = Vec::new();
    if baseline.program.project_id != candidate.program.project_id {
        reasons.push("project ID changed; existing saves belong to another project".to_owned());
    }
    if !stable_positions_removed.is_empty() {
        reasons.push("one or more explicit save positions were removed without aliases".to_owned());
    }
    if unstable_interaction_positions > 0 || unstable_call_sites > 0 {
        reasons.push(
            "changed content contains interaction or call positions without explicit stable IDs"
                .to_owned(),
        );
    }
    if !defaults_removed.is_empty() || !default_types_changed.is_empty() {
        reasons.push("saved variable structure changed".to_owned());
    }
    if changed {
        reasons.push(
            "content fingerprint changed; validate representative saves with renrs-accept --saves"
                .to_owned(),
        );
    }
    let risk = if !changed && baseline.program.project_id == candidate.program.project_id {
        "none"
    } else if baseline.program.project_id != candidate.program.project_id
        || !stable_positions_removed.is_empty()
        || unstable_interaction_positions > 0
        || unstable_call_sites > 0
    {
        "high"
    } else if !defaults_removed.is_empty() || !default_types_changed.is_empty() {
        "medium"
    } else {
        "low"
    };
    SaveCompatibility {
        risk,
        requires_save_fixture_validation: changed,
        reasons,
        defaults_added,
        defaults_removed,
        default_types_changed,
        stable_positions_removed,
        stable_positions_aliased,
        unstable_interaction_positions,
        unstable_call_sites,
    }
}

fn stable_position_changes(
    baseline: &Program,
    candidate: &Program,
) -> (Vec<String>, Vec<StableAlias>) {
    let mut removed = Vec::new();
    let mut aliased = Vec::new();
    let mut ids = baseline.explicit_instruction_ids.iter().collect::<Vec<_>>();
    ids.sort_by_key(|id| id.as_str());
    for id in ids {
        match candidate.canonical_instruction_id(id) {
            Some(current) if candidate.explicit_instruction_ids.contains(current) => {
                if current != id {
                    aliased.push(StableAlias {
                        from: id.to_string(),
                        to: current.to_string(),
                    });
                }
            }
            _ => removed.push(id.to_string()),
        }
    }
    (removed, aliased)
}

fn unstable_count(program: &Program, predicate: impl Fn(&InstructionKind) -> bool) -> usize {
    program
        .instructions
        .iter()
        .filter(|instruction| {
            predicate(&instruction.kind)
                && !program.explicit_instruction_ids.contains(&instruction.id)
        })
        .count()
}
