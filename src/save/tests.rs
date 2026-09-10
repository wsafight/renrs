use std::fs;

use crate::{Runtime, compile, parse_script};

use super::{SaveError, SaveMetadata, SaveRepository};

#[test]
fn round_trips_a_snapshot() {
    let temporary = tempfile::tempdir().unwrap();
    let repository = SaveRepository::new(temporary.path());
    let program =
        compile(&parse_script("label start:\n    \"Hello\"", "test.rns").unwrap()).unwrap();
    let mut runtime = Runtime::new(program).unwrap();
    runtime.advance().unwrap();

    repository.save("quick", &runtime.snapshot()).unwrap();
    let loaded = repository.load("quick").unwrap();
    assert_eq!(
        loaded.snapshot.stage.dialogue.as_ref().unwrap().text,
        "Hello"
    );
    assert_eq!(repository.list().unwrap()[0].name, "quick");
}

#[test]
fn rejects_path_like_slot_names() {
    let repository = SaveRepository::new("unused");
    assert!(matches!(
        repository.load("../secret"),
        Err(SaveError::InvalidSlot(_))
    ));
}

#[test]
fn detects_corruption_and_keeps_the_slot_visible() {
    let temporary = tempfile::tempdir().unwrap();
    let repository = SaveRepository::new(temporary.path());
    let program =
        compile(&parse_script("label start:\n    \"Hello\"", "test.rns").unwrap()).unwrap();
    let mut runtime = Runtime::new(program).unwrap();
    runtime.advance().unwrap();
    repository.save("slot-1", &runtime.snapshot()).unwrap();

    let path = temporary.path().join("slot-1.json");
    let mut encoded = fs::read_to_string(&path).unwrap();
    encoded = encoded.replace("Hello", "Changed");
    fs::write(&path, encoded).unwrap();

    assert!(matches!(
        repository.load("slot-1"),
        Err(SaveError::Checksum(_))
    ));
    let slots = repository.list().unwrap();
    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0].name, "slot-1");

    fs::write(&path, "{not-json").unwrap();
    assert!(repository.list().unwrap()[0].corrupt);
}

#[test]
fn rotates_quick_saves_and_exports_them() {
    let temporary = tempfile::tempdir().unwrap();
    let repository = SaveRepository::new(temporary.path().join("saves"));
    let program =
        compile(&parse_script("label start:\n    \"Hello\"", "test.rns").unwrap()).unwrap();
    let mut runtime = Runtime::new(program).unwrap();
    runtime.advance().unwrap();
    for _ in 0..3 {
        repository
            .save_rotating("quick", 3, &runtime.snapshot(), SaveMetadata::default())
            .unwrap();
    }
    assert!(repository.load("quick-3").is_ok());
    let exported = temporary.path().join("exported.json");
    repository.export("quick-1", &exported).unwrap();
    repository.import(&exported, "imported").unwrap();
    assert!(repository.load("imported").is_ok());
}

#[test]
fn rejects_missing_container_version_or_checksum() {
    let temporary = tempfile::tempdir().unwrap();
    let repository = SaveRepository::new(temporary.path());
    let program =
        compile(&parse_script("label start:\n    \"Hello\"", "test.rns").unwrap()).unwrap();
    let snapshot = Runtime::new(program).unwrap().snapshot();
    repository.save("legacy", &snapshot).unwrap();

    let path = temporary.path().join("legacy.json");
    let original: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for field in ["container_version", "checksum_sha256"] {
        let mut encoded = original.clone();
        encoded.as_object_mut().unwrap().remove(field);
        fs::write(&path, serde_json::to_vec(&encoded).unwrap()).unwrap();
        assert!(matches!(
            repository.load("legacy"),
            Err(SaveError::Format(_))
        ));
        assert!(matches!(
            repository.import(&path, "imported"),
            Err(SaveError::Format(_))
        ));
    }
    let mut encoded = original;
    encoded["checksum_sha256"] = serde_json::json!("");
    fs::write(&path, serde_json::to_vec(&encoded).unwrap()).unwrap();
    assert!(matches!(
        repository.load("legacy"),
        Err(SaveError::Checksum(_))
    ));
    assert!(matches!(
        repository.import(&path, "imported"),
        Err(SaveError::Checksum(_))
    ));
    assert!(!temporary.path().join("imported.json").exists());
}

#[test]
fn rejects_unknown_container_versions_on_load_and_import() {
    let temporary = tempfile::tempdir().unwrap();
    let repository = SaveRepository::new(temporary.path());
    let program = compile(&parse_script("label start:\n    return", "test.rns").unwrap()).unwrap();
    repository
        .save("future", &Runtime::new(program).unwrap().snapshot())
        .unwrap();
    let path = temporary.path().join("future.json");
    let mut encoded: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for version in [1, 99] {
        encoded["container_version"] = serde_json::json!(version);
        fs::write(&path, serde_json::to_vec(&encoded).unwrap()).unwrap();

        assert!(matches!(
            repository.load("future"),
            Err(SaveError::UnsupportedContainerVersion {
                found,
                current: 2
            }) if found == version
        ));
        assert!(matches!(
            repository.import(&path, "imported"),
            Err(SaveError::UnsupportedContainerVersion {
                found,
                current: 2
            }) if found == version
        ));
        assert!(!temporary.path().join("imported.json").exists());
    }
}
