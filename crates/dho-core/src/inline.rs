// SPDX-License-Identifier: MPL-2.0

use std::error::Error;
use std::fmt;

const HEADER_SIZE: usize = 4;
const ENTRY_SIZE: usize = 8;

/// One physical block range declared by an inline archive table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InlineBlockEntry {
    pub offset: u32,
    pub stored_size: u32,
}

/// A block table stored at the beginning of the same file as its MWC blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineBlockTable {
    pub entries: Vec<InlineBlockEntry>,
    pub byte_len: usize,
}

impl InlineBlockTable {
    pub fn parse(bytes: &[u8]) -> Result<Self, InlineBlockTableError> {
        if bytes.len() < HEADER_SIZE {
            return Err(InlineBlockTableError::HeaderTruncated {
                actual_len: bytes.len(),
            });
        }
        let entry_count = u32::from_le_bytes(bytes[..4].try_into().expect("four checked bytes"));
        let entry_count = usize::try_from(entry_count)
            .map_err(|_| InlineBlockTableError::TableSizeOverflow { entry_count })?;
        let byte_len = entry_count
            .checked_mul(ENTRY_SIZE)
            .and_then(|entries| HEADER_SIZE.checked_add(entries))
            .ok_or(InlineBlockTableError::TableSizeOverflow {
                entry_count: u32::try_from(entry_count).unwrap_or(u32::MAX),
            })?;
        if bytes.len() < byte_len {
            return Err(InlineBlockTableError::EntriesTruncated {
                entry_count: u32::try_from(entry_count).unwrap_or(u32::MAX),
                expected_len: byte_len,
                actual_len: bytes.len(),
            });
        }

        let mut entries = Vec::with_capacity(entry_count);
        for index in 0..entry_count {
            let start = HEADER_SIZE + index * ENTRY_SIZE;
            let offset = u32::from_le_bytes(
                bytes[start..start + 4]
                    .try_into()
                    .expect("entry range checked by table length"),
            );
            let stored_size = u32::from_le_bytes(
                bytes[start + 4..start + 8]
                    .try_into()
                    .expect("entry range checked by table length"),
            );
            let end = u64::from(offset) + u64::from(stored_size);
            if end > bytes.len() as u64 {
                return Err(InlineBlockTableError::EntryOutOfBounds {
                    index: u32::try_from(index).unwrap_or(u32::MAX),
                    offset,
                    stored_size,
                    file_size: bytes.len(),
                });
            }
            entries.push(InlineBlockEntry {
                offset,
                stored_size,
            });
        }

        Ok(Self { entries, byte_len })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InlineBlockTableError {
    HeaderTruncated {
        actual_len: usize,
    },
    TableSizeOverflow {
        entry_count: u32,
    },
    EntriesTruncated {
        entry_count: u32,
        expected_len: usize,
        actual_len: usize,
    },
    EntryOutOfBounds {
        index: u32,
        offset: u32,
        stored_size: u32,
        file_size: usize,
    },
}

impl fmt::Display for InlineBlockTableError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HeaderTruncated { actual_len } => write!(
                formatter,
                "inline block table header is truncated: expected 4 bytes, found {actual_len}"
            ),
            Self::TableSizeOverflow { entry_count } => write!(
                formatter,
                "inline block table size overflows this platform: entries={entry_count}"
            ),
            Self::EntriesTruncated {
                entry_count,
                expected_len,
                actual_len,
            } => write!(
                formatter,
                "inline block table is truncated for {entry_count} entries: expected {expected_len} bytes, found {actual_len}"
            ),
            Self::EntryOutOfBounds {
                index,
                offset,
                stored_size,
                file_size,
            } => write!(
                formatter,
                "inline block entry {index} is outside the file: offset={offset}, stored_size={stored_size}, file_size={file_size}"
            ),
        }
    }
}

impl Error for InlineBlockTableError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(entries: &[(u32, u32)], file_size: usize) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(file_size);
        bytes.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        for (offset, stored_size) in entries {
            bytes.extend_from_slice(&offset.to_le_bytes());
            bytes.extend_from_slice(&stored_size.to_le_bytes());
        }
        bytes.resize(file_size, 0);
        bytes
    }

    #[test]
    fn parses_count_offsets_and_stored_sizes() {
        let bytes = table(&[(20, 10), (30, 12)], 42);

        let parsed = InlineBlockTable::parse(&bytes).expect("parse inline table");

        assert_eq!(parsed.byte_len, 20);
        assert_eq!(
            parsed.entries,
            [
                InlineBlockEntry {
                    offset: 20,
                    stored_size: 10,
                },
                InlineBlockEntry {
                    offset: 30,
                    stored_size: 12,
                },
            ]
        );
    }

    #[test]
    fn rejects_a_truncated_header_or_entry_region() {
        assert_eq!(
            InlineBlockTable::parse(&[1, 0, 0]).unwrap_err(),
            InlineBlockTableError::HeaderTruncated { actual_len: 3 }
        );
        assert_eq!(
            InlineBlockTable::parse(&[2, 0, 0, 0, 8, 0, 0, 0]).unwrap_err(),
            InlineBlockTableError::EntriesTruncated {
                entry_count: 2,
                expected_len: 20,
                actual_len: 8,
            }
        );
    }

    #[test]
    fn rejects_an_entry_outside_the_file() {
        let bytes = table(&[(12, 9)], 20);

        assert_eq!(
            InlineBlockTable::parse(&bytes).unwrap_err(),
            InlineBlockTableError::EntryOutOfBounds {
                index: 0,
                offset: 12,
                stored_size: 9,
                file_size: 20,
            }
        );
    }
}
