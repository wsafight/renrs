use std::io::{self, BufRead, Write};

use crate::tooling::document_diagnostics;
use serde_json::{Value, json};

use super::protocol::{notify, read_message, respond, respond_error, write_message};
use super::workspace::{Workspace, workspace_root};

/// Serves the editor protocol on standard input and output.
/// # Errors
/// Returns malformed protocol or input/output errors.
pub fn run_stdio() -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    run(&mut stdin.lock(), &mut stdout.lock())
}

fn run(reader: &mut impl BufRead, writer: &mut impl Write) -> Result<(), String> {
    let mut service = Service::default();
    while let Some(message) = read_message(reader).map_err(|error| error.to_string())? {
        if service.dispatch(writer, &message)? {
            break;
        }
    }
    Ok(())
}

#[derive(Default)]
struct Service {
    workspace: Workspace,
}

impl Service {
    fn dispatch(&mut self, writer: &mut impl Write, message: &Value) -> Result<bool, String> {
        let method = message.get("method").and_then(Value::as_str).unwrap_or("");
        let id = message.get("id").cloned();
        match method {
            "initialize" => self.initialize(writer, message, id)?,
            "shutdown" => respond(writer, id, &Value::Null)?,
            "exit" => return Ok(true),
            "textDocument/didOpen" => self.did_open(writer, message)?,
            "textDocument/didChange" => self.did_change(writer, message)?,
            "textDocument/didClose" => self.did_close(writer, message)?,
            "workspace/didChangeWatchedFiles" => self.workspace.refresh(),
            _ if id.is_some() => self.request(writer, method, message, id)?,
            _ => {}
        }
        Ok(false)
    }

    fn initialize(
        &mut self,
        writer: &mut impl Write,
        message: &Value,
        id: Option<Value>,
    ) -> Result<(), String> {
        if let Some(root) = workspace_root(message) {
            self.workspace.load(&root);
        }
        respond(
            writer,
            id,
            &json!({
                "capabilities": {
                    "textDocumentSync": 1,
                    "documentSymbolProvider": true,
                    "workspaceSymbolProvider": true,
                    "documentFormattingProvider": true,
                    "definitionProvider": true,
                    "referencesProvider": true,
                    "renameProvider": {"prepareProvider": false},
                    "completionProvider": {"triggerCharacters": [" "]}
                },
                "serverInfo": {
                    "name": "renrs-lsp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        )
    }

    fn request(
        &self,
        writer: &mut impl Write,
        method: &str,
        message: &Value,
        id: Option<Value>,
    ) -> Result<(), String> {
        let uri = message
            .pointer("/params/textDocument/uri")
            .and_then(Value::as_str)
            .unwrap_or("");
        match method {
            "workspace/symbol" => respond(
                writer,
                id,
                &Value::Array(self.workspace.workspace_symbols()),
            ),
            "textDocument/documentSymbol" => respond(
                writer,
                id,
                &Value::Array(self.workspace.document_symbols(uri)),
            ),
            "textDocument/formatting" => respond(
                writer,
                id,
                &Value::Array(self.workspace.formatting_edits(uri)),
            ),
            "textDocument/definition" => respond(
                writer,
                id,
                &Value::Array(self.workspace.definitions(message)),
            ),
            "textDocument/references" => respond(
                writer,
                id,
                &Value::Array(self.workspace.references(message)),
            ),
            "textDocument/rename" => self.rename(writer, message, id),
            "textDocument/completion" => {
                respond(writer, id, &Value::Array(self.workspace.completions()))
            }
            _ => write_message(
                writer,
                &json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {"code": -32601, "message": "method not found"}
                }),
            ),
        }
    }

    fn rename(
        &self,
        writer: &mut impl Write,
        message: &Value,
        id: Option<Value>,
    ) -> Result<(), String> {
        let new_name = message
            .pointer("/params/newName")
            .and_then(Value::as_str)
            .unwrap_or("");
        match self.workspace.rename(message, new_name) {
            Some(edit) => respond(writer, id, &edit),
            None => respond_error(writer, id, -32602, "newName must be a valid identifier"),
        }
    }

    fn did_open(&mut self, writer: &mut impl Write, message: &Value) -> Result<(), String> {
        if let Some(document) = message.pointer("/params/textDocument")
            && let (Some(uri), Some(source)) = (
                document.get("uri").and_then(Value::as_str),
                document.get("text").and_then(Value::as_str),
            )
        {
            self.workspace.insert(uri, source);
            publish_diagnostics(writer, uri, source)?;
        }
        Ok(())
    }

    fn did_change(&mut self, writer: &mut impl Write, message: &Value) -> Result<(), String> {
        let uri = message
            .pointer("/params/textDocument/uri")
            .and_then(Value::as_str);
        let source = message
            .pointer("/params/contentChanges/0/text")
            .and_then(Value::as_str);
        if let (Some(uri), Some(source)) = (uri, source) {
            self.workspace.insert(uri, source);
            publish_diagnostics(writer, uri, source)?;
        }
        Ok(())
    }

    fn did_close(&mut self, writer: &mut impl Write, message: &Value) -> Result<(), String> {
        if let Some(uri) = message
            .pointer("/params/textDocument/uri")
            .and_then(Value::as_str)
        {
            self.workspace.remove(uri);
            notify(
                writer,
                "textDocument/publishDiagnostics",
                &json!({"uri": uri, "diagnostics": []}),
            )?;
        }
        Ok(())
    }
}

fn publish_diagnostics(writer: &mut impl Write, uri: &str, source: &str) -> Result<(), String> {
    let diagnostics = document_diagnostics(source, uri)
        .into_iter()
        .map(|diagnostic| {
            let line = diagnostic.line.saturating_sub(1);
            let column = diagnostic.column.saturating_sub(1);
            json!({
                "range": {
                    "start": {"line": line, "character": column},
                    "end": {"line": line, "character": column + 1}
                },
                "severity": 1,
                "source": "renrs",
                "message": diagnostic.message
            })
        })
        .collect::<Vec<_>>();
    notify(
        writer,
        "textDocument/publishDiagnostics",
        &json!({"uri": uri, "diagnostics": diagnostics}),
    )
}
