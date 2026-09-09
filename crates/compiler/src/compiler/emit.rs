use crate::syntax::StatementKind;

use super::InstructionKind;

pub(super) fn simple_instruction(kind: &StatementKind) -> Option<InstructionKind> {
    Some(match kind {
        StatementKind::Window { visible } => InstructionKind::Window { visible: *visible },
        StatementKind::ShowScreen { name } => InstructionKind::ShowScreen { name: name.clone() },
        StatementKind::HideScreen { name } => InstructionKind::HideScreen { name: name.clone() },
        StatementKind::CallScreen { name } => InstructionKind::CallScreen { name: name.clone() },
        StatementKind::Video { path, seconds } => InstructionKind::Video {
            path: path.clone(),
            seconds: *seconds,
        },
        StatementKind::PlayMusic {
            path,
            repeat,
            fade_in,
            volume,
            if_changed,
        } => InstructionKind::PlayMusic {
            path: path.clone(),
            repeat: *repeat,
            fade_in: *fade_in,
            volume: *volume,
            if_changed: *if_changed,
        },
        StatementKind::QueueMusic {
            path,
            repeat,
            fade_in,
            volume,
        } => InstructionKind::QueueMusic {
            path: path.clone(),
            repeat: *repeat,
            fade_in: *fade_in,
            volume: *volume,
        },
        StatementKind::PlaySound {
            path,
            volume,
            repeat,
        } => InstructionKind::PlaySound {
            path: path.clone(),
            volume: *volume,
            repeat: *repeat,
        },
        StatementKind::QueueSound {
            path,
            volume,
            repeat,
        } => InstructionKind::QueueSound {
            path: path.clone(),
            volume: *volume,
            repeat: *repeat,
        },
        StatementKind::PlayVoice { path } => InstructionKind::PlayVoice { path: path.clone() },
        StatementKind::StopMusic { fade_out } => InstructionKind::StopMusic {
            fade_out: *fade_out,
        },
        StatementKind::StopSound { fade_out } => InstructionKind::StopSound {
            fade_out: *fade_out,
        },
        StatementKind::StopVoice { fade_out } => InstructionKind::StopVoice {
            fade_out: *fade_out,
        },
        StatementKind::Pause { seconds } => InstructionKind::Pause { seconds: *seconds },
        StatementKind::Nvl { mode } => InstructionKind::Nvl { mode: mode.clone() },
        StatementKind::Extension {
            name,
            variable,
            input,
        } => InstructionKind::Extension {
            name: name.clone(),
            variable: variable.clone(),
            input: input.clone(),
        },
        StatementKind::Parallel { tracks } => InstructionKind::Parallel {
            tracks: tracks.clone(),
        },
        _ => return None,
    })
}
