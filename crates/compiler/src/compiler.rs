#[path = "compiler/calls.rs"]
mod calls;
#[path = "compiler/display.rs"]
mod display;
#[path = "compiler/ids.rs"]
mod ids;
#[path = "compiler/lower.rs"]
mod lower;
#[path = "compiler/model.rs"]
mod model;

use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;
use sha2::{Digest, Sha256};

use crate::syntax::{Script, Span};

use ids::stable_statement_id;
use lower::Compiler;
pub use model::{
    ChoicePrompt, ChoiceTarget, CompileError, Instruction, InstructionId, InstructionKind, Program,
    StatementId,
};

/// Compiles a validated syntax tree into executable, flat instructions.
///
/// # Errors
///
/// Returns an error when a referenced label is missing, a stable ID collides,
/// or the syntax tree cannot be serialized for its deterministic fingerprint.
pub fn compile(script: &Script) -> Result<Program, CompileError> {
    if let Some(parameter) = script
        .label_parameters
        .get("start")
        .and_then(|parameters| parameters.first())
    {
        return Err(CompileError::ParameterizedStart {
            file: parameter.span.source.clone(),
            line: parameter.span.line,
        });
    }
    let mut compiler = Compiler {
        instructions: Vec::new(),
        labels: IndexMap::new(),
        unresolved: Vec::new(),
        current_label: String::new(),
        label_parameters: &script.label_parameters,
        instruction_aliases: HashMap::new(),
        alias_collision: None,
        explicit_instruction_ids: HashSet::new(),
    };
    for (name, block) in &script.labels {
        compiler.current_label.clone_from(name);
        compiler
            .labels
            .insert(name.clone(), compiler.instructions.len());
        compiler.block(block, "");
        let span = block.first().map_or_else(
            || Span::in_source(&script.source_name, 1, 1),
            |statement| statement.span.clone(),
        );
        let statement_id = stable_statement_id(&span.source, name, "label-end");
        compiler.emit(
            span,
            statement_id,
            "implicit-return",
            InstructionKind::Return { value: None },
        );
    }
    compiler.resolve()?;
    resolve_images(&mut compiler.instructions, script)?;
    let display_layers = resolve_display_layers(&mut compiler.instructions, script)?;
    if let Some(alias) = compiler.alias_collision.clone() {
        return Err(CompileError::DuplicateInstructionId(alias));
    }

    let mut instruction_by_id = HashMap::new();
    for (index, instruction) in compiler.instructions.iter().enumerate() {
        if instruction_by_id
            .insert(instruction.id.clone(), index)
            .is_some()
        {
            return Err(CompileError::DuplicateInstructionId(instruction.id.clone()));
        }
    }
    for alias in compiler.instruction_aliases.keys() {
        if instruction_by_id.contains_key(alias) {
            return Err(CompileError::DuplicateInstructionId(alias.clone()));
        }
    }
    validate_translation_ids(&compiler.instructions)?;

    let encoded = serde_json::to_vec(script).map_err(CompileError::Fingerprint)?;
    let fingerprint = format!("{:x}", Sha256::digest(encoded));
    Ok(Program {
        extensions: std::collections::BTreeMap::default(),
        layered_images: std::collections::BTreeMap::default(),
        progress: crate::progress::ProgressConfig::default(),
        title: script.title.clone(),
        project_id: script.project_id.clone(),
        fingerprint,
        characters: script.characters.clone(),
        defaults: script.defaults.clone(),
        display_layers,
        instructions: compiler.instructions,
        labels: compiler.labels,
        label_parameters: script
            .label_parameters
            .iter()
            .map(|(label, parameters)| {
                (
                    label.clone(),
                    parameters
                        .iter()
                        .map(|parameter| parameter.name.clone())
                        .collect(),
                )
            })
            .collect(),
        instruction_by_id,
        instruction_aliases: compiler.instruction_aliases,
        explicit_instruction_ids: compiler.explicit_instruction_ids,
    })
}

fn resolve_display_layers(
    instructions: &mut [Instruction],
    script: &Script,
) -> Result<std::collections::BTreeMap<String, i32>, CompileError> {
    let mut layers = std::collections::BTreeMap::from([
        ("master".to_owned(), 0),
        ("transient".to_owned(), 100),
        ("screens".to_owned(), 200),
        ("overlay".to_owned(), 300),
    ]);
    layers.extend(
        script
            .display_layers
            .iter()
            .map(|(name, definition)| (name.clone(), definition.order)),
    );
    for instruction in instructions {
        let (name, order) = match &mut instruction.kind {
            InstructionKind::Show {
                display_layer,
                display_order,
                ..
            } => (display_layer, Some(display_order)),
            InstructionKind::ClearLayer { display_layer } => (display_layer, None),
            _ => continue,
        };
        let Some(resolved) = layers.get(name) else {
            return Err(CompileError::UnknownDisplayLayer {
                name: name.clone(),
                file: instruction.span.source.clone(),
                line: instruction.span.line,
            });
        };
        if let Some(order) = order {
            *order = *resolved;
        }
    }
    Ok(layers)
}

fn validate_translation_ids(instructions: &[Instruction]) -> Result<(), CompileError> {
    let mut seen = HashSet::new();
    for id in instructions
        .iter()
        .flat_map(|instruction| match &instruction.kind {
            InstructionKind::Dialogue { translation_id, .. } => vec![translation_id],
            InstructionKind::Choice { prompt, options } => prompt
                .iter()
                .map(|prompt| &prompt.translation_id)
                .chain(options.iter().map(|option| &option.translation_id))
                .collect(),
            _ => Vec::new(),
        })
    {
        if !seen.insert(id) {
            return Err(CompileError::DuplicateTranslationId(id.clone()));
        }
    }
    Ok(())
}

fn resolve_images(instructions: &mut [Instruction], script: &Script) -> Result<(), CompileError> {
    for instruction in instructions {
        let (InstructionKind::Scene { path } | InstructionKind::Show { path, .. }) =
            &mut instruction.kind
        else {
            continue;
        };
        let Some(name) = path.strip_prefix("@image:") else {
            continue;
        };
        let Some(image) = script.images.get(name) else {
            return Err(CompileError::UnknownImage {
                name: name.to_owned(),
                file: instruction.span.source.clone(),
                line: instruction.span.line,
            });
        };
        path.clone_from(&image.path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::localization::TranslationId;
    use crate::parse_script;
    use ids::{stable_anchored_statement_id, stable_instruction_id};

    #[test]
    fn compiles_menu_targets_and_labels() {
        let script = parse_script(
            "label start:\n    menu \"Choose\":\n        \"A\":\n            jump end\n        \"B\":\n            \"B\"\nlabel end:\n    return",
            "test.rns",
        )
        .unwrap();
        let program = compile(&script).unwrap();
        assert_eq!(program.labels.len(), 2);
        let InstructionKind::Choice { prompt, options } = &program.instructions[0].kind else {
            panic!("first instruction should be a choice");
        };
        assert_eq!(
            prompt.as_ref().map(|prompt| prompt.text.as_str()),
            Some("Choose")
        );
        assert_eq!(options.len(), 2);
        assert_ne!(options[0].target, options[1].target);
        assert_eq!(program.instruction_by_id.len(), program.instructions.len());
    }

    #[test]
    fn ids_survive_line_and_dialogue_edits() {
        let first = compile(
            &parse_script("label start:\n    \"Before\"\n    return", "story/main.rns").unwrap(),
        )
        .unwrap();
        let second = compile(
            &parse_script(
                "\n\nlabel start:\n    \"After editing the words\"\n    return",
                "story/main.rns",
            )
            .unwrap(),
        )
        .unwrap();

        let first_ids = first
            .instructions
            .iter()
            .map(|instruction| instruction.id.clone())
            .collect::<Vec<_>>();
        let second_ids = second
            .instructions
            .iter()
            .map(|instruction| instruction.id.clone())
            .collect::<Vec<_>>();
        assert_eq!(first_ids, second_ids);
        assert_ne!(first.fingerprint, second.fingerprint);
    }

    #[test]
    fn explicit_anchor_produces_old_id_alias() {
        let program = compile(
            &parse_script(
                "label start:\n    @id \"line.new\" alias \"line.old\" \"Hello\"",
                "story.rns",
            )
            .unwrap(),
        )
        .unwrap();
        let old_statement = stable_anchored_statement_id(&TranslationId::new("line.old").unwrap());
        let old = stable_instruction_id(&old_statement, "main");
        assert_eq!(program.instruction_index(&old), Some(0));
    }

    #[test]
    fn normalizes_named_and_default_call_arguments() {
        let program = compile(
            &parse_script(
                "label start:\n    call target(third=30, first=10)\nlabel target(first, second=20, third=3):\n    return",
                "test.rns",
            )
            .unwrap(),
        )
        .unwrap();
        let InstructionKind::Call {
            arguments,
            parameters,
            ..
        } = &program.instructions[0].kind
        else {
            panic!("expected call instruction");
        };
        assert_eq!(parameters, &["first", "second", "third"]);
        let values = arguments
            .iter()
            .map(|argument| match argument {
                crate::syntax::Expr::Value(crate::syntax::Value::Integer(value)) => *value,
                _ => panic!("expected integer argument"),
            })
            .collect::<Vec<_>>();
        assert_eq!(values, [10, 20, 30]);
    }

    #[test]
    fn rejects_invalid_call_bindings() {
        let cases = [
            (
                "call target",
                "label target(required):\n    return",
                "missing required argument `required`",
            ),
            (
                "call target(1, 2)",
                "label target(only):\n    return",
                "at most 1 positional arguments",
            ),
            (
                "call target(other=1)",
                "label target(known=1):\n    return",
                "no parameter named `other`",
            ),
            (
                "call target(1, first=2)",
                "label target(first):\n    return",
                "parameter `first` more than once",
            ),
        ];
        for (call, target, expected) in cases {
            let source = format!("label start:\n    {call}\n{target}");
            let error = compile(&parse_script(&source, "test.rns").unwrap()).unwrap_err();
            assert!(
                error.to_string().contains(expected),
                "unexpected error: {error}"
            );
        }
    }

    #[test]
    fn rejects_parameterized_start_and_jump_entries() {
        let start =
            compile(&parse_script("label start(value=1):\n    return", "test.rns").unwrap())
                .unwrap_err();
        assert!(matches!(start, CompileError::ParameterizedStart { .. }));

        let jump = compile(
            &parse_script(
                "label start:\n    jump target\nlabel target(value=1):\n    return",
                "test.rns",
            )
            .unwrap(),
        )
        .unwrap_err();
        assert!(matches!(jump, CompileError::ParameterizedJump { .. }));
    }

    #[test]
    fn resolves_named_display_layers_and_rejects_unknown_ones() {
        let program = compile(
            &parse_script(
                "layer effects order 50\nlabel start:\n    show \"hero.png\" onlayer effects zorder 7\n    clear effects",
                "test.rns",
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(program.display_layers["effects"], 50);
        assert!(matches!(
            &program.instructions[0].kind,
            InstructionKind::Show { display_layer, display_order: 50, layer: 7, .. }
                if display_layer == "effects"
        ));

        let error = compile(
            &parse_script(
                "label start:\n    show \"hero.png\" onlayer missing",
                "test.rns",
            )
            .unwrap(),
        )
        .unwrap_err();
        assert!(matches!(error, CompileError::UnknownDisplayLayer { .. }));
    }
}
