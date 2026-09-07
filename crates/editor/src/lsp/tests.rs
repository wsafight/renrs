use std::fs;
use std::io::Cursor;

use crate::tooling::{SymbolRole, symbol_at};
use serde_json::json;

use super::protocol::{read_message, write_message};
use super::workspace::{Workspace, file_uri_path, path_uri};

#[test]
fn reads_and_writes_lsp_frames() {
    let message = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"});
    let mut encoded = Vec::new();
    write_message(&mut encoded, &message).unwrap();
    let decoded = read_message(&mut Cursor::new(encoded)).unwrap().unwrap();
    assert_eq!(decoded, message);
}

#[test]
fn loads_workspace_documents_and_resolves_symbols() {
    let temporary = tempfile::tempdir().unwrap();
    fs::write(
        temporary.path().join("main.rns"),
        "label start:\n    call ending\n",
    )
    .unwrap();
    fs::write(
        temporary.path().join("ending.rns"),
        "label ending:\n    return\n",
    )
    .unwrap();
    let mut workspace = Workspace::default();
    workspace.load(temporary.path());
    assert_eq!(workspace.len(), 2);
    let reference = workspace
        .source(&path_uri(&temporary.path().join("main.rns")))
        .and_then(|source| symbol_at(source, 1, 10))
        .unwrap();
    let matches = workspace.occurrences(&reference);
    assert_eq!(matches.len(), 2);
    assert!(
        matches
            .iter()
            .any(|(_, item)| item.role == SymbolRole::Definition)
    );
    assert!(
        workspace
            .completions()
            .iter()
            .any(|item| item["label"] == "volume")
    );
    let path = temporary.path().join("a file.rns");
    let uri = path_uri(&path);
    assert_eq!(file_uri_path(&uri), Some(path));
}

#[test]
fn closing_and_external_changes_preserve_correct_disk_index() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("a file.rns");
    fs::write(&path, "label disk:\n    return\n").unwrap();
    let uri = path_uri(&path);
    let mut workspace = Workspace::default();
    workspace.load(root.path());
    workspace.insert(&uri, "label buffer:\n    return\n");
    workspace.refresh();
    assert!(workspace.source(&uri).unwrap().contains("buffer"));
    workspace.remove(&uri);
    assert!(workspace.source(&uri).unwrap().contains("disk"));
    fs::write(&path, "label changed:\n    return\n").unwrap();
    workspace.refresh();
    assert!(workspace.source(&uri).unwrap().contains("changed"));
    fs::remove_file(path).unwrap();
    workspace.refresh();
    assert!(workspace.source(&uri).is_none());
}
