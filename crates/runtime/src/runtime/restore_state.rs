use super::{Program, RuntimeError, StageState, VisualEffect, WaitState};

pub(super) fn prepare_saved_stage(
    program: &Program,
    stage: &mut StageState,
) -> Result<(), RuntimeError> {
    if stage.music.iter().chain(&stage.music_queue).any(|track| {
        !track.fade_in.is_finite()
            || track.fade_in < 0.0
            || !track.volume.is_finite()
            || !(0.0..=1.0).contains(&track.volume)
    }) {
        return Err(RuntimeError::InvalidWaitState);
    }
    for sprite in &mut stage.sprites {
        let Some(order) = program.display_layers.get(&sprite.display_layer) else {
            return Err(RuntimeError::SavedDisplayLayerMissing(
                sprite.display_layer.clone(),
            ));
        };
        sprite.display_order = *order;
    }
    Ok(())
}

pub(super) fn prepare_saved_wait(
    program: &Program,
    waiting: &mut WaitState,
) -> Result<(), RuntimeError> {
    if let WaitState::Effect {
        effect: VisualEffect::Parallel { from, .. } | VisualEffect::Dissolve { from, .. },
    } = waiting
    {
        prepare_saved_stage(program, from)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        compile, parse_script,
        runtime::{MusicState, Runtime},
    };
    use std::sync::Arc;

    #[test]
    fn rejects_out_of_range_audio_gain_in_saved_stages() {
        let program =
            compile(&parse_script("label start:\n    return", "test.rns").unwrap()).unwrap();
        let invalid = StageState {
            music: Some(MusicState {
                path: "audio/theme.ogg".to_owned(),
                repeat: false,
                fade_in: 0.0,
                volume: 1.1,
            }),
            ..StageState::default()
        };
        let mut snapshot = Runtime::new(program.clone()).unwrap().snapshot();
        snapshot.stage = Arc::new(invalid.clone());
        assert!(matches!(
            Runtime::restore(program.clone(), snapshot),
            Err(RuntimeError::InvalidWaitState)
        ));
        let mut waiting = WaitState::Effect {
            effect: VisualEffect::Dissolve {
                from: Box::new(invalid),
                seconds: 0.5,
            },
        };
        assert!(matches!(
            prepare_saved_wait(&program, &mut waiting),
            Err(RuntimeError::InvalidWaitState)
        ));
    }
}
