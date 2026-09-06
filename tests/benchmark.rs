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
