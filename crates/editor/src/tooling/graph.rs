use std::collections::HashSet;
use std::fmt::Write;

use crate::syntax::{Block, Script, StatementKind};

#[must_use]
pub fn story_graph(script: &Script) -> String {
    let mut edges = Vec::new();
    for (label, block) in &script.labels {
        collect_edges(label, block, &mut edges);
    }

    let mut output = String::from("digraph renrs_story {\n");
    output.push_str("  rankdir=LR;\n  node [shape=box];\n");
    for label in script.labels.keys() {
        let escaped = dot_escape(label);
        let _ = writeln!(output, "  \"{escaped}\";");
    }
    let mut unique = HashSet::new();
    for (from, to, kind) in edges {
        if unique.insert((from, to, kind)) {
            let _ = writeln!(
                output,
                "  \"{}\" -> \"{}\" [label=\"{}\"];",
                dot_escape(from),
                dot_escape(to),
                kind
            );
        }
    }
    output.push_str("}\n");
    output
}

fn collect_edges<'a>(
    from: &'a str,
    block: &'a Block,
    edges: &mut Vec<(&'a str, &'a str, &'static str)>,
) {
    for statement in block {
        match &statement.kind {
            StatementKind::Jump { label } => edges.push((from, label, "jump")),
            StatementKind::Call { label, .. } => edges.push((from, label, "call")),
            StatementKind::If {
                branches,
                else_block,
            } => {
                for (_, branch) in branches {
                    collect_edges(from, branch, edges);
                }
                collect_edges(from, else_block, edges);
            }
            StatementKind::Menu { options, .. } => {
                for option in options {
                    collect_edges(from, &option.block, edges);
                }
            }
            _ => {}
        }
    }
}

fn dot_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
