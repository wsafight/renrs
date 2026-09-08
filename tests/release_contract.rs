use renrs::protocol::PROTOCOL_VERSION;
use renrs::runtime::RuntimeSnapshot;
use renrs::save_format::SaveFile;
use renrs::screens::Screens;

#[test]
fn checked_release_contract_matches_compiled_formats() {
    let contract: serde_json::Value = serde_json::from_slice(include_bytes!("../release.json"))
        .expect("release.json must be valid JSON");
    assert_eq!(contract["engine_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(contract["formats"]["machine_protocol"], PROTOCOL_VERSION);
    assert_eq!(
        contract["formats"]["snapshot"],
        RuntimeSnapshot::FORMAT_VERSION
    );
    assert_eq!(
        contract["formats"]["save_container"],
        SaveFile::CONTAINER_VERSION
    );
    assert_eq!(contract["formats"]["screens"], Screens::default().version);
}
