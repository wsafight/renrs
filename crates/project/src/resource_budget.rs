use std::io::{self, Read};

pub const SCRIPT_BYTES: usize = 16 * 1024 * 1024;
pub const TOTAL_SCRIPT_BYTES: usize = 64 * 1024 * 1024;
pub const RESOURCE_RULES_BYTES: usize = 64 * 1024;
pub const CONFIG_BYTES: usize = 4 * 1024 * 1024;
pub const CATALOG_BYTES: usize = 16 * 1024 * 1024;
pub const VIDEO_MANIFEST_BYTES: usize = 64 * 1024 * 1024;

pub fn read_limited(reader: impl Read, limit: usize) -> io::Result<Vec<u8>> {
    let limit = u64::try_from(limit).unwrap_or(u64::MAX);
    let mut bytes = Vec::new();
    reader
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if bytes.len() > usize::try_from(limit).unwrap_or(usize::MAX) {
        return Err(io::Error::other("encoded resource exceeds byte budget"));
    }
    Ok(bytes)
}

pub fn reserve(total: &mut usize, additional: usize, limit: usize) -> io::Result<()> {
    let next = total
        .checked_add(additional)
        .filter(|next| *next <= limit)
        .ok_or_else(|| io::Error::other("resource collection exceeds byte budget"))?;
    *total = next;
    Ok(())
}

pub fn decode_script(bytes: Vec<u8>, total: &mut usize) -> io::Result<String> {
    reserve(total, bytes.len(), TOTAL_SCRIPT_BYTES)?;
    String::from_utf8(bytes).map_err(io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_reads_and_totals_reject_overflow() {
        assert_eq!(read_limited(&[1, 2, 3][..], 3).unwrap(), [1, 2, 3]);
        assert!(read_limited(&[1, 2, 3, 4][..], 3).is_err());
        let mut total = 6;
        reserve(&mut total, 2, 8).unwrap();
        assert_eq!(total, 8);
        assert!(reserve(&mut total, 1, 8).is_err());
        assert!(reserve(&mut total, usize::MAX, usize::MAX).is_err());
    }
}
