use crate::{runtime::StageState, syntax::AnimationStep};

#[cfg(test)]
#[path = "animation/tests.rs"]
mod camera_tests;

/// Samples independent tracks against their immutable starting stage.
#[must_use]
pub fn sample(from: &StageState, tracks: &[Vec<AnimationStep>], elapsed: f32) -> StageState {
    let mut stage = from.clone();
    for track in tracks {
        let mut remaining = elapsed.max(0.0);
        for step in track {
            if let AnimationStep::Transform {
                alias,
                properties,
                seconds,
                easing,
            } = step
            {
                let transform = if alias == "camera" {
                    Some(&mut stage.camera)
                } else {
                    stage
                        .sprites
                        .iter_mut()
                        .find(|sprite| &sprite.alias == alias)
                        .map(|sprite| &mut sprite.transform)
                };
                let Some(transform) = transform else { continue };
                let target = properties.apply(*transform);
                let progress = if *seconds <= f32::EPSILON {
                    1.0
                } else {
                    easing.sample(remaining / seconds)
                };
                *transform = transform.interpolate(target, progress);
            }
            if remaining < step.seconds() {
                break;
            }
            remaining -= step.seconds();
        }
    }
    stage
}

/// Validates referenced sprites before committing the final stage.
/// # Errors
/// Rejects missing aliases and invalid/conflicting animation tracks.
pub fn validate(from: &StageState, tracks: &[Vec<AnimationStep>]) -> Result<f32, String> {
    let duration = crate::syntax::validate_tracks(tracks)?;
    for step in tracks.iter().flatten() {
        if let AnimationStep::Transform { alias, .. } = step
            && alias != "camera"
            && !from.sprites.iter().any(|sprite| &sprite.alias == alias)
        {
            return Err(format!("cannot transform unknown image alias `{alias}`"));
        }
    }
    Ok(duration)
}

#[cfg(test)]
mod tests {
    use crate::{Runtime, WaitState, compile, parse_script, runtime::VisualEffect};
    #[test]
    fn independent_tracks_sample_and_restore_at_the_same_time() {
        let script = "label start:\n    show \"a.png\" as a\n    show \"b.png\" as b\n    parallel:\n        timeline:\n            transform a x 100 over 2\n        timeline:\n            pause 1\n            transform b alpha 0 over 1\n    \"Done\"\n";
        let program = compile(&parse_script(script, "parallel.rns").unwrap()).unwrap();
        let mut runtime = Runtime::new(program.clone()).unwrap();
        let WaitState::Effect {
            effect:
                VisualEffect::Parallel {
                    from,
                    tracks,
                    seconds,
                },
        } = runtime.advance().unwrap()
        else {
            panic!("parallel wait required")
        };
        assert!((seconds - 2.0).abs() < f32::EPSILON);
        let midway = super::sample(&from, &tracks, 1.5);
        assert!((midway.sprites[0].transform.x - 75.0).abs() < 0.001);
        assert!((midway.sprites[1].transform.alpha - 0.5).abs() < 0.001);
        let restored = Runtime::restore(program, runtime.snapshot()).unwrap();
        assert_eq!(restored.waiting(), runtime.waiting());
        assert!(
            parse_script(
                &script.replace("transform b", "transform a"),
                "conflict.rns"
            )
            .is_err()
        );
    }
}
