use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use renrs::localization::{TranslationCatalog, TranslationId, extract_catalog};
use renrs::{ProjectSource, analyze, compile};

fn main() {
    let arguments = env::args_os().skip(1).collect::<Vec<_>>();
    let result = match arguments.as_slice() {
        [command, project, language, output] if command == "extract" => {
            extract(project, language, output)
        }
        [command, project, catalog] if command == "update" => update(project, catalog),
        [command, project, catalog] if command == "check" => check(project, catalog),
        _ => Err(usage()),
    };
    if let Err(error) = result {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn extract(
    project: &std::ffi::OsStr,
    language: &std::ffi::OsStr,
    output: &std::ffi::OsStr,
) -> Result<(), String> {
    let program = load_program(Path::new(project))?;
    let messages = extract_catalog(&program)
        .into_iter()
        .map(|entry| (entry.id, entry.text))
        .collect();
    let catalog = TranslationCatalog::new(language.to_string_lossy(), None, messages)
        .map_err(|error| error.to_string())?;
    write_catalog(Path::new(output), &catalog)?;
    println!(
        "Extracted {} messages to {}",
        catalog.messages.len(),
        Path::new(output).display()
    );
    Ok(())
}

fn update(project: &std::ffi::OsStr, catalog_path: &std::ffi::OsStr) -> Result<(), String> {
    let program = load_program(Path::new(project))?;
    let path = Path::new(catalog_path);
    let mut catalog = TranslationCatalog::load(path).map_err(|error| error.to_string())?;
    let source = source_messages(&program);
    let before = catalog.messages.len();
    for id in source.keys() {
        if !catalog.plurals.contains_key(id) {
            catalog.messages.entry(id.clone()).or_default();
        }
    }
    write_catalog(path, &catalog)?;
    println!(
        "Updated {}: {} added, {} obsolete retained",
        path.display(),
        catalog.messages.len().saturating_sub(before),
        catalog
            .messages
            .keys()
            .filter(|id| !source.contains_key(*id))
            .count()
    );
    Ok(())
}

fn check(project: &std::ffi::OsStr, catalog_path: &std::ffi::OsStr) -> Result<(), String> {
    let program = load_program(Path::new(project))?;
    let path = Path::new(catalog_path);
    let catalog = TranslationCatalog::load(path).map_err(|error| error.to_string())?;
    let source = source_messages(&program);
    let missing = source
        .keys()
        .filter(|id| {
            !catalog.plurals.contains_key(*id)
                && catalog.messages.get(*id).is_none_or(String::is_empty)
        })
        .cloned()
        .collect::<BTreeSet<_>>();
    let obsolete = catalog
        .messages
        .keys()
        .chain(catalog.plurals.keys())
        .filter(|id| !source.contains_key(*id))
        .cloned()
        .collect::<BTreeSet<_>>();
    for id in &missing {
        eprintln!("missing: {id}");
    }
    for id in &obsolete {
        eprintln!("obsolete: {id}");
    }
    if missing.is_empty() {
        println!(
            "OK: {} translated messages, {} obsolete",
            source.len(),
            obsolete.len()
        );
        Ok(())
    } else {
        Err(format!("{} messages are untranslated", missing.len()))
    }
}

fn load_program(path: &Path) -> Result<renrs::Program, String> {
    let source = ProjectSource::open(path.to_path_buf()).map_err(|error| error.to_string())?;
    let script = source
        .load_script()
        .map_err(|diagnostics| format_diagnostics(&diagnostics))?;
    let diagnostics = source.validate(&script);
    if !diagnostics.is_empty() {
        return Err(format_diagnostics(&diagnostics));
    }
    let program = compile(&script).map_err(|error| error.to_string())?;
    let analysis = analyze(&program);
    let errors = analysis
        .iter()
        .filter(|diagnostic| diagnostic.is_error())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if errors.is_empty() {
        Ok(program)
    } else {
        Err(errors.join("\n"))
    }
}

fn source_messages(program: &renrs::Program) -> BTreeMap<TranslationId, String> {
    extract_catalog(program)
        .into_iter()
        .map(|entry| (entry.id, entry.text))
        .collect()
}

fn write_catalog(path: &Path, catalog: &TranslationCatalog) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let temporary = temporary_path(path);
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temporary)
        .map_err(|error| error.to_string())?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, catalog).map_err(|error| error.to_string())?;
    writer.write_all(b"\n").map_err(|error| error.to_string())?;
    writer.flush().map_err(|error| error.to_string())?;
    writer
        .get_ref()
        .sync_all()
        .map_err(|error| error.to_string())?;
    replace_file(&temporary, path).map_err(|error| error.to_string())
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut value = path.as_os_str().to_owned();
    value.push(format!(".{}.tmp", std::process::id()));
    PathBuf::from(value)
}

fn replace_file(temporary: &Path, destination: &Path) -> std::io::Result<()> {
    if let Err(error) = fs::rename(temporary, destination) {
        if destination.exists() {
            fs::remove_file(destination)?;
            fs::rename(temporary, destination)
        } else {
            Err(error)
        }
    } else {
        Ok(())
    }
}

fn format_diagnostics(diagnostics: &[renrs::Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

fn usage() -> String {
    "usage: renrs-i18n extract <project> <language> <catalog.json>\n       renrs-i18n update <project> <catalog.json>\n       renrs-i18n check <project> <catalog.json>".to_owned()
}
