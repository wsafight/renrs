use std::env;
use std::path::PathBuf;

use renrs::archive::pack_project;

fn main() {
    let mut arguments = env::args_os().skip(1);
    let Some(root) = arguments.next().map(PathBuf::from) else {
        fail("usage: renrs-pack <game-directory> <output.renrs>");
    };
    let Some(output) = arguments.next().map(PathBuf::from) else {
        fail("usage: renrs-pack <game-directory> <output.renrs>");
    };
    if arguments.next().is_some() {
        fail("usage: renrs-pack <game-directory> <output.renrs>");
    }
    match pack_project(&root, &output) {
        Ok(_) => println!("packed {}", output.display()),
        Err(error) => fail(error.to_string()),
    }
}

fn fail(message: impl AsRef<str>) -> ! {
    eprintln!("error: {}", message.as_ref());
    std::process::exit(1);
}
