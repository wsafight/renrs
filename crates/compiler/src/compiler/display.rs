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
                at_transform,
            } => {
                if let Some(name) = at_transform {
                    let Some(definition) = self.transforms.get(name) else {
                        self.error = Some(super::CompileError::UnknownTransform {
                            name: name.clone(),
                            file: span.source.clone(),
                            line: span.line,
                        });
                        return;
                    };
                    let properties = definition.properties;
                    self.emit(
                        span.clone(),
                        statement_id.clone(),
                        "main",
                        InstructionKind::Show {
                            path: path.clone(),
                            alias: alias.clone(),
                            position: *position,
                            layer: *layer,
                            display_layer: display_layer.clone(),
                            display_order: 0,
                        },
                    );
                    self.emit(
                        span.clone(),
                        statement_id.clone(),
                        "at-transform",
                        InstructionKind::Transform {
                            alias: alias.clone(),
                            properties,
                            seconds: 0.0,
                            easing: crate::syntax::Easing::Linear,
                        },
                    );
                    return;
                }
                InstructionKind::Show {
                    path: path.clone(),
                    alias: alias.clone(),
                    position: *position,
                    layer: *layer,
                    display_layer: display_layer.clone(),
                    display_order: 0,
                }
            }
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
