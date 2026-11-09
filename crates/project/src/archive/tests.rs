use super::*;

fn write_archive(path: &Path, files: &[(&str, &[u8])]) {
    let mut offset = 0_u64;
    let entries = files
        .iter()
        .map(|(path, bytes)| {
            let entry = ArchiveEntry {
                path: (*path).to_owned(),
                offset,
                length: bytes.len() as u64,
                sha256: renrs_shared::hash::sha256_hex(bytes),
            };
            offset += bytes.len() as u64;
            entry
        })
        .collect();
    let manifest = serde_json::to_vec(&Manifest {
        version: FORMAT_VERSION,
        entries,
    })
    .unwrap();
    let mut output = File::create(path).unwrap();
    output.write_all(MAGIC).unwrap();
    output
        .write_all(&u64::try_from(manifest.len()).unwrap().to_le_bytes())
        .unwrap();
    output.write_all(&manifest).unwrap();
    for (_, bytes) in files {
        output.write_all(bytes).unwrap();
    }
}

#[test]
fn packs_reads_and_extracts_project_files() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("game");
    fs::create_dir_all(root.join("images")).unwrap();
    fs::write(root.join("script.rns"), b"label start:\n    return\n").unwrap();
    fs::write(root.join("images/pixel.bin"), [1, 2, 3, 4]).unwrap();
    fs::create_dir(root.join(".renrs")).unwrap();
    fs::write(root.join(".renrs/private.json"), b"secret").unwrap();
    let archive_path = temporary.path().join("game.renrs");

    pack_project(&root, &archive_path).unwrap();
    let archive = ResourceArchive::open(&archive_path).unwrap();
    assert_eq!(archive.entries().len(), 2);
    assert!(archive.contains("script.rns"));
    assert!(!archive.contains("missing.rns"));
    assert_eq!(archive.read("images/pixel.bin").unwrap(), [1, 2, 3, 4]);
    assert!(matches!(
        archive.read(".renrs/private.json"),
        Err(ArchiveError::Missing(_))
    ));

    let extracted = temporary.path().join("extracted");
    archive.extract(&extracted).unwrap();
    assert_eq!(
        fs::read(extracted.join("script.rns")).unwrap(),
        b"label start:\n    return\n"
    );
}

#[test]
fn third_party_archives_use_directory_resource_rules() {
    let temporary = tempfile::tempdir().unwrap();
    let archive_path = temporary.path().join("game.renrs");
    write_archive(
        &archive_path,
        &[
            ("script.rns", b"label start:\n    return\n"),
            ("story/visible.rns", b"label visible:\n    return\n"),
            ("target/extra.rns", b"label extra:\n    return\n"),
            ("drafts/excluded.rns", b"label excluded:\n    return\n"),
            (".hidden.rns", b"label hidden:\n    return\n"),
            ("resources.json", br#"{"exclude":["drafts/"]}"#),
        ],
    );

    let archive = ResourceArchive::open(&archive_path).unwrap();
    assert_eq!(archive.entries().len(), 3);
    assert!(!archive.contains("target/extra.rns"));
    assert!(matches!(
        archive.read("drafts/excluded.rns"),
        Err(ArchiveError::Missing(_))
    ));
    let script = crate::project::load_project_archive(&archive).unwrap();
    assert_eq!(script.labels.len(), 2);
    assert!(script.labels.contains_key("visible"));

    let extracted = temporary.path().join("extracted");
    archive.extract(&extracted).unwrap();
    assert!(!extracted.join("target").exists());
    assert!(!extracted.join("drafts").exists());
    assert!(!extracted.join(".hidden.rns").exists());
}

#[test]
fn repacks_only_visible_entries() {
    let temporary = tempfile::tempdir().unwrap();
    let source = temporary.path().join("game.renrs");
    write_archive(
        &source,
        &[
            ("script.rns", b"label start:\n    return\n"),
            ("drafts/excluded.rns", b"label excluded:\n    return\n"),
            (".hidden.rns", b"label hidden:\n    return\n"),
            ("resources.json", br#"{"exclude":["drafts/"]}"#),
        ],
    );
    let archive = ResourceArchive::open(&source).unwrap();
    assert_eq!(archive.entries().len(), 2);

    let destination = temporary.path().join("normalized.renrs");
    let count = archive.repack(&destination).unwrap();
    assert_eq!(count, 2);
    let repacked = ResourceArchive::open(&destination).unwrap();
    assert_eq!(repacked.entries().len(), 2);
    assert_eq!(
        repacked.read("script.rns").unwrap(),
        b"label start:\n    return\n"
    );
    assert!(matches!(
        repacked.read("drafts/excluded.rns"),
        Err(ArchiveError::Missing(_))
    ));
}

#[cfg(unix)]
#[test]
fn refuses_to_extract_through_an_existing_directory_symlink() {
    use std::os::unix::fs::symlink;

    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("game");
    fs::create_dir_all(root.join("images")).unwrap();
    fs::write(root.join("script.rns"), b"label start:\n    return\n").unwrap();
    fs::write(root.join("images/pixel.bin"), [1, 2, 3, 4]).unwrap();
    let archive_path = temporary.path().join("game.renrs");
    pack_project(&root, &archive_path).unwrap();

    let destination = temporary.path().join("extracted");
    let outside = temporary.path().join("outside");
    fs::create_dir_all(&destination).unwrap();
    fs::create_dir_all(&outside).unwrap();
    symlink(&outside, destination.join("images")).unwrap();

    let archive = ResourceArchive::open(&archive_path).unwrap();
    assert!(matches!(
        archive.extract(&destination),
        Err(ArchiveError::Symlink(_))
    ));
    assert!(!outside.join("pixel.bin").exists());
}
