use std::env;
use std::path::PathBuf;

use renrs::archive::ResourceArchive;

fn main() {
    let mut arguments = env::args_os().skip(1);
    let Some(archive_path) = arguments.next().map(PathBuf::from) else {
        fail("usage: renrs-unpack <archive.renrs> <output-directory>");
    };
    let Some(output) = arguments.next().map(PathBuf::from) else {
        fail("usage: renrs-unpack <archive.renrs> <output-directory>");
    };
    if arguments.next().is_some() {
        fail("usage: renrs-unpack <archive.renrs> <output-directory>");
    }
    let archive =
        ResourceArchive::open(&archive_path).unwrap_or_else(|error| fail(error.to_string()));
    archive
        .extract(&output)
        .unwrap_or_else(|error| fail(error.to_string()));
    println!("unpacked {} entries", archive.entries().len());
}

fn fail(message: impl AsRef<str>) -> ! {
    eprintln!("error: {}", message.as_ref());
    std::process::exit(1);
}
