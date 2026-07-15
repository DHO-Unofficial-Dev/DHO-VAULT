// SPDX-License-Identifier: MPL-2.0

use std::error::Error;
use std::fmt;

/// Byte length of an indexed image archive header.
pub const HEADER_SIZE: usize = 28;

/// Byte length of one indexed image archive record.
pub const RECORD_SIZE: usize = 20;

/// Raw fields from the 28-byte little-endian archive header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveHeader {
    pub record_count: u32,
    pub group_count: u32,
    pub default_width: u32,
    pub default_height: u32,
    pub image_block_count: u32,
    pub archive_count: u32,
    pub reserved: u32,
}

/// Raw fields from one 20-byte little-endian index record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexRecord {
    pub icon_id: u32,
    pub block_index: u32,
    pub width: u32,
    pub height: u32,
    pub group_code: u32,
}

/// A parsed index file, including bytes whose meaning is not yet known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedArchive {
    pub header: ArchiveHeader,
    pub records: Vec<IndexRecord>,
    pub trailing_index_bytes: usize,
}

/// Failures that can be identified without interpreting archive semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexParseError {
    HeaderTooShort {
        actual_len: usize,
    },
    RecordRegionSizeOverflow {
        record_count: u32,
    },
    RecordRegionTruncated {
        record_count: u32,
        required_len: usize,
        actual_len: usize,
    },
}

impl fmt::Display for IndexParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HeaderTooShort { actual_len } => write!(
                formatter,
                "index header is truncated: need at least {HEADER_SIZE} bytes, found {actual_len}"
            ),
            Self::RecordRegionSizeOverflow { record_count } => write!(
                formatter,
                "index record region size overflows this platform: record_count={record_count}"
            ),
            Self::RecordRegionTruncated {
                record_count,
                required_len,
                actual_len,
            } => write!(
                formatter,
                "index record region is truncated: record_count={record_count}, need {required_len} bytes, found {actual_len}"
            ),
        }
    }
}

impl Error for IndexParseError {}

impl IndexedArchive {
    /// Parses a complete index file while preserving all raw record fields.
    pub fn parse(bytes: &[u8]) -> Result<Self, IndexParseError> {
        if bytes.len() < HEADER_SIZE {
            return Err(IndexParseError::HeaderTooShort {
                actual_len: bytes.len(),
            });
        }

        let header = ArchiveHeader {
            record_count: read_u32(bytes, 0),
            group_count: read_u32(bytes, 4),
            default_width: read_u32(bytes, 8),
            default_height: read_u32(bytes, 12),
            image_block_count: read_u32(bytes, 16),
            archive_count: read_u32(bytes, 20),
            reserved: read_u32(bytes, 24),
        };

        let record_count = usize::try_from(header.record_count).map_err(|_| {
            IndexParseError::RecordRegionSizeOverflow {
                record_count: header.record_count,
            }
        })?;
        let record_bytes = record_count.checked_mul(RECORD_SIZE).ok_or(
            IndexParseError::RecordRegionSizeOverflow {
                record_count: header.record_count,
            },
        )?;
        let records_end = HEADER_SIZE.checked_add(record_bytes).ok_or(
            IndexParseError::RecordRegionSizeOverflow {
                record_count: header.record_count,
            },
        )?;

        if bytes.len() < records_end {
            return Err(IndexParseError::RecordRegionTruncated {
                record_count: header.record_count,
                required_len: records_end,
                actual_len: bytes.len(),
            });
        }

        let mut records = Vec::with_capacity(record_count);
        for record_bytes in bytes[HEADER_SIZE..records_end].chunks_exact(RECORD_SIZE) {
            records.push(IndexRecord {
                icon_id: read_u32(record_bytes, 0),
                block_index: read_u32(record_bytes, 4),
                width: read_u32(record_bytes, 8),
                height: read_u32(record_bytes, 12),
                group_code: read_u32(record_bytes, 16),
            });
        }

        Ok(Self {
            header,
            records,
            trailing_index_bytes: bytes.len() - records_end,
        })
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + size_of::<u32>()]
            .try_into()
            .expect("the caller validated the fixed-size record"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_u32(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn sample_index() -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [2, 1, 48, 48, 2, 1, 0] {
            push_u32(&mut bytes, value);
        }
        for value in [1181, 7, 48, 48, 99] {
            push_u32(&mut bytes, value);
        }
        for value in [1182, 8, 48, 48, 99] {
            push_u32(&mut bytes, value);
        }
        bytes.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
        bytes
    }

    #[test]
    fn parses_header_records_and_trailing_bytes() {
        let parsed = IndexedArchive::parse(&sample_index()).expect("valid sample index");

        assert_eq!(
            parsed.header,
            ArchiveHeader {
                record_count: 2,
                group_count: 1,
                default_width: 48,
                default_height: 48,
                image_block_count: 2,
                archive_count: 1,
                reserved: 0,
            }
        );
        assert_eq!(
            parsed.records,
            [
                IndexRecord {
                    icon_id: 1181,
                    block_index: 7,
                    width: 48,
                    height: 48,
                    group_code: 99,
                },
                IndexRecord {
                    icon_id: 1182,
                    block_index: 8,
                    width: 48,
                    height: 48,
                    group_code: 99,
                },
            ]
        );
        assert_eq!(parsed.trailing_index_bytes, 3);
    }

    #[test]
    fn rejects_a_truncated_header() {
        let error = IndexedArchive::parse(&[0; HEADER_SIZE - 1]).unwrap_err();

        assert_eq!(
            error,
            IndexParseError::HeaderTooShort {
                actual_len: HEADER_SIZE - 1,
            }
        );
    }

    #[test]
    fn rejects_a_truncated_record_region() {
        let mut bytes = sample_index();
        bytes.truncate(HEADER_SIZE + RECORD_SIZE * 2 - 1);

        let error = IndexedArchive::parse(&bytes).unwrap_err();

        assert_eq!(
            error,
            IndexParseError::RecordRegionTruncated {
                record_count: 2,
                required_len: HEADER_SIZE + RECORD_SIZE * 2,
                actual_len: HEADER_SIZE + RECORD_SIZE * 2 - 1,
            }
        );
    }
}
