use crate::syntax::{AnimationStep, Block, StatementKind, validate_tracks};

pub(super) fn parallel_tracks(block: Block) -> Result<StatementKind, String> {
    let tracks = block
        .into_iter()
        .map(|statement| {
            let StatementKind::Timeline { block } = statement.kind else {
                return Err("parallel requires timeline blocks".to_owned());
            };
            block
                .into_iter()
                .map(|step| match step.kind {
                    StatementKind::Transform {
                        alias,
                        properties,
                        seconds,
                        easing,
                    } => Ok(AnimationStep::Transform {
                        alias,
                        properties,
                        seconds,
                        easing,
                    }),
                    StatementKind::Pause { seconds } => Ok(AnimationStep::Pause { seconds }),
                    _ => Err("parallel timelines support transform and pause".to_owned()),
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    validate_tracks(&tracks)?;
    Ok(StatementKind::Parallel { tracks })
}
