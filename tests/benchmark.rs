#[test]
fn generated_workload_checks_routes_and_archives_consistently() {
    let root = tempfile::tempdir().unwrap();
    let game = root.path().join("game");
    renrs::benchmark::generate(&game, 3, 30).unwrap();
    let source = renrs::ProjectSource::open(&game).unwrap();
    let program = source.compile().unwrap();
    let suite: renrs::debugger::RouteSuite =
        serde_json::from_slice(&std::fs::read(game.join("routes.json")).unwrap()).unwrap();
    for route in &suite.routes {
        renrs::debugger::run_route(&program, route, 1000).unwrap();
    }
    let report = renrs::benchmark::measure(&game, 1).unwrap();
    assert!(report.interactions > 90);
    assert!(report.snapshot_bytes > 1000);
    assert!(report.archive_bytes > 1000);
}

#[test]
fn first_party_reference_has_a_fixed_acceptance_profile() {
    let temporary = tempfile::tempdir().unwrap();
    let game = temporary.path().join("reference");
    renrs::benchmark::generate_reference(&game).unwrap();
    let profile: serde_json::Value =
        serde_json::from_slice(&std::fs::read(game.join("reference.json")).unwrap()).unwrap();
    assert_eq!(profile["kind"], "first_party_reference_fixture");
    assert_eq!(profile["estimated_reading_minutes"]["minimum"], 30);
    assert_eq!(profile["estimated_reading_minutes"]["maximum"], 60);
    assert_eq!(profile["dialogue_lines"], 500);
    assert_eq!(profile["external_author_validation"], false);
    let program = renrs::ProjectSource::open(&game)
        .unwrap()
        .compile()
        .unwrap();
    assert_eq!(program.instructions.len(), 585);
}
