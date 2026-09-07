use crate::syntax::{Span, StatementKind};

use super::{Compiler, InstructionKind, StatementId};

impl Compiler<'_> {
    pub(super) fn lower_display(
        &mut self,
        statement: &StatementKind,
        span: &Span,
        statement_id: &StatementId,
    ) {
        let kind = match statement {
            StatementKind::Scene { path } => InstructionKind::Scene { path: path.clone() },
            StatementKind::Show {
                path,
                alias,
                position,
                layer,
                display_layer,
            } => InstructionKind::Show {
                path: path.clone(),
                alias: alias.clone(),
                position: *position,
                layer: *layer,
                display_layer: display_layer.clone(),
                display_order: 0,
            },
            StatementKind::Hide { alias } => InstructionKind::Hide {
                alias: alias.clone(),
            },
            StatementKind::ClearLayer { display_layer } => InstructionKind::ClearLayer {
                display_layer: display_layer.clone(),
            },
            _ => unreachable!("display lowering requires a display statement"),
        };
        self.emit(span.clone(), statement_id.clone(), "main", kind);
    }
}
