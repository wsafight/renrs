use super::*;
use std::path::Path;

#[test]
fn safe_relative_path_accepts_normal_segments_only() {
    assert!(path::safe_relative_path("images/a.png"));
    assert!(!path::safe_relative_path(""));
    assert!(!path::safe_relative_path("/etc/passwd"));
    assert!(!path::safe_relative_path("../secret"));
    assert!(!path::safe_relative_path("a/../../b"));
}

#[test]
fn safe_resource_path_also_rejects_windows_forms() {
    assert!(path::safe_resource_path("images/a.png"));
    assert!(!path::safe_resource_path("images\\a.png"));
    assert!(!path::safe_resource_path("C:/images/a.png"));
    assert!(!path::safe_resource_path("images//a.png"));
    assert!(!path::safe_resource_path("./a.png"));
}

#[test]
fn relative_name_uses_slash_separators_and_falls_back() {
    let root = Path::new("/game");
    assert_eq!(
        path::relative_name(root, Path::new("/game/a/b.png")),
        "a/b.png"
    );
    // A path outside `root` keeps its own components; the root component
    // renders as "/", so the fallback is the joined absolute path.
    assert_eq!(
        path::relative_name(root, Path::new("/other/c.png")),
        "//other/c.png"
    );
    assert_eq!(path::relative_name(root, Path::new("d.png")), "d.png");
}

#[test]
#[cfg(feature = "hash")]
fn hash_helpers_agree_on_the_same_bytes() {
    let bytes = b"renrs";
    let expected = hash::sha256_hex(bytes);
    assert_eq!(hash::sha256_reader(&bytes[..]).unwrap(), expected);
    assert_eq!(expected.len(), 64);
}

#[test]
#[cfg(feature = "hash")]
fn copy_hashed_reports_length_and_digest() {
    let mut input: &[u8] = b"payload";
    let mut output = Vec::new();
    let mut buffer = [0_u8; 4];
    let (length, digest) = hash::copy_hashed(&mut input, &mut output, &mut buffer).unwrap();
    assert_eq!(length, 7);
    assert_eq!(output, b"payload");
    assert_eq!(digest, hash::sha256_hex(b"payload"));
}

#[test]
#[cfg(feature = "hash")]
fn copy_hashed_rejects_an_empty_buffer() {
    let mut input: &[u8] = b"payload";
    let mut output = Vec::new();
    let error = hash::copy_hashed(&mut input, &mut output, &mut []).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(output, Vec::<u8>::new());
}

#[test]
#[cfg(feature = "hash")]
fn fold_fingerprint_changes_with_the_payload() {
    let first = hash::fold_fingerprint("base", &[1, 2, 3]).unwrap();
    let second = hash::fold_fingerprint("base", &[1, 2, 4]).unwrap();
    assert_ne!(first, second);
    assert_eq!(first, hash::fold_fingerprint("base", &[1, 2, 3]).unwrap());
}

#[test]
#[cfg(feature = "io")]
fn write_json_round_trips_through_read_json() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("nested/value.json");
    io::write_json(&file, &vec![1, 2, 3]).unwrap();
    assert_eq!(io::read_json::<Vec<u8>>(&file).unwrap(), [1, 2, 3]);
}

#[test]
#[cfg(feature = "io")]
fn replace_file_moves_over_an_existing_destination() {
    let directory = tempfile::tempdir().unwrap();
    let temporary = directory.path().join("new");
    let destination = directory.path().join("old");
    std::fs::write(&temporary, "new").unwrap();
    std::fs::write(&destination, "old").unwrap();
    io::replace_file(&temporary, &destination).unwrap();
    assert_eq!(std::fs::read_to_string(&destination).unwrap(), "new");
}

#[test]
#[cfg(feature = "io")]
fn failed_replace_file_preserves_an_existing_destination() {
    let directory = tempfile::tempdir().unwrap();
    let missing = directory.path().join("missing");
    let destination = directory.path().join("old");
    std::fs::write(&destination, "old").unwrap();
    assert!(io::replace_file(&missing, &destination).is_err());
    assert_eq!(std::fs::read_to_string(&destination).unwrap(), "old");
}

#[test]
fn identifier_and_color_predicates() {
    assert!(text::is_identifier("mira_2"));
    assert!(!text::is_identifier("2mira"));
    assert!(!text::is_identifier(""));
    assert!(text::is_hex_color("#ff0080"));
    assert!(text::is_hex_color("#ff008080"));
    assert!(!text::is_hex_color("ff0080"));
    assert_eq!(text::parse_hex_color("#ff0080"), Some([255, 0, 128, 255]));
    assert_eq!(text::parse_hex_color("#ff008080"), Some([255, 0, 128, 128]));
}

#[test]
fn ui_key_is_stable_across_surfaces() {
    assert_eq!(text::ui_key("New Game"), "ui.new_game");
    assert_eq!(text::ui_key("  Load   Save "), "ui.load_save");
}

#[test]
fn join_diagnostics_separates_entries() {
    assert_eq!(diagnostic::join_diagnostics(&["a", "b"]), "a\nb");
    assert_eq!(diagnostic::join_diagnostics::<&str>(&[]), "");
}
