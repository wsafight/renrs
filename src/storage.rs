pub use renrs_shared::io::{atomic_write, read_json, replace_file, write_json};

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Write};

    #[test]
    fn interrupted_write_preserves_previous_file() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("save.json");
        write_json(&path, &"previous").unwrap();
        assert!(
            atomic_write(&path, |file| {
                file.write_all(b"partial")?;
                Err(io::Error::other("injected failure"))
            })
            .is_err()
        );
        assert_eq!(std::fs::read_to_string(path).unwrap(), "\"previous\"\n");
    }
}
