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

fn messages(input: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let mut encoded = Vec::new();
    for message in input {
        write_message(&mut encoded, message).unwrap();
    }
    let mut output = Vec::new();
    super::service::run(&mut Cursor::new(encoded), &mut output).unwrap();
    let mut reader = Cursor::new(output);
    let mut messages = Vec::new();
    while let Some(message) = read_message(&mut reader).unwrap() {
        messages.push(message);
    }
    messages
}

#[test]
fn lsp_session_formats_completes_defines_renames_and_shuts_down() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("script.rns"),
        "label start:\n    jump end\nlabel end:\n    return\n",
    )
    .unwrap();
    let uri = path_uri(&root.path().join("script.rns"));
    let replies = messages(&[
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"rootUri": path_uri(root.path())}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"text":"label start:\n    jump end\nlabel end:\n    return\n"}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/documentSymbol","params":{"textDocument":{"uri":uri}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"workspace/symbol","params":{"query":"start"}}),
        json!({"jsonrpc":"2.0","id":4,"method":"textDocument/formatting","params":{"textDocument":{"uri":uri}}}),
        json!({"jsonrpc":"2.0","id":5,"method":"textDocument/definition","params":{"textDocument":{"uri":uri},"position":{"line":1,"character":9}}}),
        json!({"jsonrpc":"2.0","id":6,"method":"textDocument/references","params":{"textDocument":{"uri":uri},"position":{"line":2,"character":6}}}),
        json!({"jsonrpc":"2.0","id":7,"method":"textDocument/completion","params":{"textDocument":{"uri":uri}}}),
        json!({"jsonrpc":"2.0","id":8,"method":"textDocument/rename","params":{"textDocument":{"uri":uri},"position":{"line":2,"character":6},"newName":"done"}}),
        json!({"jsonrpc":"2.0","id":9,"method":"textDocument/rename","params":{"textDocument":{"uri":uri},"position":{"line":2,"character":6},"newName":"1bad"}}),
        json!({"jsonrpc":"2.0","id":10,"method":"textDocument/nope"}),
        json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":uri},"contentChanges":[{"text":"label start:\n    return\n"}]}}),
        json!({"jsonrpc":"2.0","method":"workspace/didChangeWatchedFiles","params":{"changes":[]}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":uri}}}),
        json!({"jsonrpc":"2.0","id":11,"method":"shutdown"}),
        json!({"jsonrpc":"2.0","method":"exit"}),
    ]);
    assert!(replies.iter().any(|item| item.get("id") == Some(&json!(1))));
    assert!(
        replies
            .iter()
            .any(|item| item.get("method") == Some(&json!("textDocument/publishDiagnostics")))
    );
    assert!(
        replies
            .iter()
            .any(|item| item.get("id") == Some(&json!(8)) && item.get("result").is_some())
    );
    assert!(replies.iter().any(|item| {
        item.get("id") == Some(&json!(9))
            && item["error"]["message"]
                .as_str()
                .is_some_and(|message| message.contains("identifier"))
    }));
    assert!(
        replies
            .iter()
            .any(|item| { item.get("id") == Some(&json!(10)) && item["error"]["code"] == -32601 })
    );
}
