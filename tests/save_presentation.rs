use renrs::save::{SaveMetadata, SavePresentation, SaveRepository};

#[test]
fn presentation_roundtrips_and_stale_summaries_do_not_hide_corruption() {
    let root = tempfile::tempdir().unwrap();
    let repo = SaveRepository::new(root.path());
    let mut runtime = renrs::Runtime::new(
        renrs::compile(&renrs::parse_script("label start:\n    \"Hello\"", "test.rns").unwrap())
            .unwrap(),
    )
    .unwrap();
    runtime.advance().unwrap();
    repo.save_with_metadata(
        "slot-1",
        &runtime.snapshot(),
        SaveMetadata {
            presentation: Some(SavePresentation {
                sprite_elapsed_ms: 1234,
                dialogue_page: 2,
                visible_characters: 17,
                pause_remaining_ms: 345,
                effect_remaining_ms: 678,
                note: "Route A".to_owned(),
                thumbnail_png: vec![1, 2, 3],
            }),
            ..SaveMetadata::default()
        },
    )
    .unwrap();
    let view = repo.load("slot-1").unwrap().presentation.unwrap();
    assert_eq!(
        (
            view.dialogue_page,
            view.visible_characters,
            view.pause_remaining_ms,
            view.effect_remaining_ms
        ),
        (2, 17, 345, 678)
    );
    assert_eq!(repo.list_cached().unwrap()[0].note, "Route A");
    std::fs::write(root.path().join("slot-1.json"), b"broken").unwrap();
    assert!(repo.load("slot-1").is_err());
    assert!(SaveRepository::new(root.path()).list_cached().unwrap()[0].corrupt);
}
