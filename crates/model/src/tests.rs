use std::collections::{BTreeMap, HashMap, HashSet};

use indexmap::IndexMap;
use renrs_syntax::presentation::{ImageFrame, ImageLayer};
use renrs_syntax::syntax::{Expr, Value};

use super::{
    CompiledImageLayer, CompiledLayeredImage, CompiledProgram, InstructionId, ModelError,
    ProgressConfig, ProjectBundle, Unlock,
};

fn empty_program() -> CompiledProgram {
    CompiledProgram {
        title: "Test".to_owned(),
        project_id: "org.test".to_owned(),
        fingerprint: "fp".to_owned(),
        characters: IndexMap::new(),
        defaults: IndexMap::new(),
        display_layers: BTreeMap::new(),
        images: IndexMap::new(),
        transforms: IndexMap::new(),
        instructions: Vec::new(),
        labels: IndexMap::new(),
        label_parameters: IndexMap::new(),
        instruction_by_id: HashMap::new(),
        instruction_aliases: HashMap::new(),
        explicit_instruction_ids: HashSet::new(),
    }
}

fn program_with_labels(labels: &[&str]) -> CompiledProgram {
    let mut program = empty_program();
    for (index, name) in labels.iter().enumerate() {
        program.labels.insert((*name).to_owned(), index);
    }
    program
}

#[test]
fn instruction_ids_require_the_inst_hex_contract() {
    assert!(InstructionId::new("inst_ab").is_ok());
    assert!(InstructionId::new("inst_DEADBEEF").is_ok());
    assert!(InstructionId::new("inst_").is_err());
    assert!(InstructionId::new("inst_zz").is_err());
    assert!(InstructionId::new("ab").is_err());
    assert_eq!(InstructionId::new("inst_aa").unwrap().as_str(), "inst_aa");
}

#[test]
fn aliases_resolve_to_live_instructions_and_reject_conflicts() {
    let current = InstructionId::new("inst_aa").unwrap();
    let old = InstructionId::new("inst_bb").unwrap();
    let mut program = empty_program();
    program.instruction_by_id.insert(current.clone(), 3);

    program
        .add_instruction_alias(old.clone(), current.clone())
        .unwrap();
    assert_eq!(program.instruction_index(&old), Some(3));
    assert_eq!(program.instruction_index(&current), Some(3));
    assert_eq!(program.canonical_instruction_id(&old), Some(&current));
    assert_eq!(program.canonical_instruction_id(&current), Some(&current));

    let missing = InstructionId::new("inst_cc").unwrap();
    assert!(matches!(
        program.add_instruction_alias(InstructionId::new("inst_dd").unwrap(), missing),
        Err(ModelError::UnknownAliasTarget(_))
    ));
    assert!(matches!(
        program.add_instruction_alias(old, current.clone()),
        Err(ModelError::DuplicateInstructionId(_))
    ));
    assert!(matches!(
        program.add_instruction_alias(current.clone(), current),
        Err(ModelError::DuplicateInstructionId(_))
    ));
}

#[test]
fn project_bundle_derefs_to_the_compiled_program() {
    let mut program = empty_program();
    program.title = "Story".to_owned();
    let mut bundle = ProjectBundle::from(program);
    assert_eq!(bundle.title, "Story");
    bundle.title = "Edited".to_owned();
    assert_eq!(bundle.compiled.title, "Edited");
}

#[test]
fn progress_rejects_duplicate_ids_missing_labels_and_unsafe_images() {
    let program = program_with_labels(&["start", "ending"]);
    let valid = ProgressConfig {
        endings: vec![Unlock {
            id: "good".to_owned(),
            title: "Good".to_owned(),
            label: "ending".to_owned(),
            image: Some("images/end.png".to_owned()),
        }],
        rollback_barriers: vec!["start".to_owned()],
        ..ProgressConfig::default()
    };
    valid.validate(&program).unwrap();

    let duplicate = ProgressConfig {
        endings: vec![
            Unlock {
                id: "good".to_owned(),
                title: "A".to_owned(),
                label: "ending".to_owned(),
                image: None,
            },
            Unlock {
                id: "good".to_owned(),
                title: "B".to_owned(),
                label: "ending".to_owned(),
                image: None,
            },
        ],
        ..ProgressConfig::default()
    };
    assert!(
        duplicate
            .validate(&program)
            .unwrap_err()
            .contains("duplicate")
    );

    let unknown = ProgressConfig {
        gallery: vec![Unlock {
            id: "art".to_owned(),
            title: "Art".to_owned(),
            label: "missing".to_owned(),
            image: None,
        }],
        ..ProgressConfig::default()
    };
    assert!(
        unknown
            .validate(&program)
            .unwrap_err()
            .contains("unknown unlock")
    );

    let unsafe_path = ProgressConfig {
        gallery: vec![Unlock {
            id: "art".to_owned(),
            title: "Art".to_owned(),
            label: "ending".to_owned(),
            image: Some("../secret.png".to_owned()),
        }],
        ..ProgressConfig::default()
    };
    assert_eq!(
        unsafe_path.validate(&program).unwrap_err(),
        "unsafe gallery image path"
    );

    let barrier = ProgressConfig {
        rollback_barriers: vec!["gone".to_owned()],
        ..ProgressConfig::default()
    };
    assert!(
        barrier
            .validate(&program)
            .unwrap_err()
            .contains("unknown rollback barrier")
    );
}

#[test]
fn compiled_layers_expose_source_paths_and_runtime_layers() {
    let layer = CompiledImageLayer {
        path: "body.png".to_owned(),
        condition: Some(Expr::Value(Value::Boolean(true))),
        x: 4.0,
        y: 8.0,
        frames: vec![ImageFrame {
            path: "blink.png".to_owned(),
            seconds: 0.2,
        }],
        speaking: true,
    };
    assert_eq!(layer.paths().collect::<Vec<_>>(), ["body.png", "blink.png"]);
    let source = layer.source_layer();
    assert_eq!(source.path, "body.png");
    assert!(source.when.is_none());
    assert!(source.speaking);

    let compiled = CompiledLayeredImage {
        width: 100,
        height: 200,
        layers: vec![layer],
    };
    let resolved = compiled.resolved(vec![ImageLayer {
        path: "body.png".to_owned(),
        when: None,
        x: 0.0,
        y: 0.0,
        frames: Vec::new(),
        speaking: false,
    }]);
    assert_eq!(resolved.width, 100);
    assert_eq!(resolved.layers.len(), 1);
}
