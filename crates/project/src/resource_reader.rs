use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};

/// A seekable view restricted to one resource, including inside an archive.
pub struct ResourceReader {
    file: File,
    start: u64,
    length: u64,
    position: u64,
}

impl ResourceReader {
    pub(crate) fn new(mut file: File, start: u64, length: u64) -> io::Result<Self> {
        file.seek(SeekFrom::Start(start))?;
        Ok(Self {
            file,
            start,
            length,
            position: 0,
        })
    }
}

impl Read for ResourceReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let available = self
            .length
            .saturating_sub(self.position)
            .min(buffer.len() as u64);
        let count = self
            .file
            .read(&mut buffer[..usize::try_from(available).unwrap()])?;
        self.position += count as u64;
        Ok(count)
    }
}

impl Seek for ResourceReader {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let position = match from {
            SeekFrom::Start(position) => i128::from(position),
            SeekFrom::Current(offset) => i128::from(self.position) + i128::from(offset),
            SeekFrom::End(offset) => i128::from(self.length) + i128::from(offset),
        };
        let position = u64::try_from(position)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid resource seek"))?;
        let absolute = self.start.checked_add(position).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "resource offset overflow")
        })?;
        self.file.seek(SeekFrom::Start(absolute))?;
        self.position = position;
        Ok(position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seeks_and_reads_stay_inside_an_archive_entry() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("a.bin"), b"first").unwrap();
        std::fs::write(root.path().join("b.bin"), b"second").unwrap();
        let archive = root.path().join(".game.renrs");
        crate::archive::pack_project(root.path(), &archive).unwrap();
        let source = crate::ProjectSource::open(archive).unwrap();
        let mut reader = source.open_reader("a.bin").unwrap();
        let mut data = Vec::new();
        reader.read_to_end(&mut data).unwrap();
        assert_eq!(data, b"first");
        assert_eq!(reader.seek(SeekFrom::End(-2)).unwrap(), 3);
        data.clear();
        reader.read_to_end(&mut data).unwrap();
        assert_eq!(data, b"st");
        assert!(reader.seek(SeekFrom::Start(50)).is_ok());
        assert_eq!(reader.read(&mut [0; 8]).unwrap(), 0);
        assert!(reader.seek(SeekFrom::Current(-51)).is_err());
    }
}
