// SPDX-License-Identifier: MPL-2.0

use flate2::read::ZlibDecoder;
use std::error::Error;
use std::fmt;
use std::io::Read;

/// Four-byte marker at the beginning of a compressed MWC block.
pub const MWC_MAGIC: [u8; 4] = [0x4D, 0x57, 0x43, 0x1A];

/// Byte length of a compressed MWC block header.
pub const BLOCK_HEADER_SIZE: usize = 12;

/// A byte position in one numbered data file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockLocation {
    pub file_number: u32,
    pub offset: usize,
}

/// A compressed block whose header and payload fit in the scanned file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MwcBlock {
    pub location: BlockLocation,
    pub uncompressed_size: u32,
    pub compressed_size: u32,
    pub payload_offset: usize,
}

/// Bytes that cannot yet be assigned a resource meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnresolvedGap {
    pub location: BlockLocation,
    pub len: usize,
}

/// One ordered segment of a data file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataSegment {
    ZlibBlock(MwcBlock),
    UnresolvedGap(UnresolvedGap),
}

/// Ordered scan results for one numbered data file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedDataFile {
    pub file_number: u32,
    pub file_size: usize,
    pub segments: Vec<DataSegment>,
}

impl ScannedDataFile {
    pub fn zlib_blocks(&self) -> impl Iterator<Item = &MwcBlock> {
        self.segments.iter().filter_map(|segment| match segment {
            DataSegment::ZlibBlock(block) => Some(block),
            DataSegment::UnresolvedGap(_) => None,
        })
    }

    pub fn unresolved_gaps(&self) -> impl Iterator<Item = &UnresolvedGap> {
        self.segments.iter().filter_map(|segment| match segment {
            DataSegment::ZlibBlock(_) => None,
            DataSegment::UnresolvedGap(gap) => Some(gap),
        })
    }
}

/// Structural failures found while walking a data file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockScanError {
    HeaderTruncated {
        location: BlockLocation,
        remaining_bytes: usize,
    },
    PayloadEndOverflow {
        location: BlockLocation,
        compressed_size: u32,
    },
    PayloadTruncated {
        location: BlockLocation,
        compressed_size: u32,
        available_bytes: usize,
    },
}

impl fmt::Display for BlockScanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HeaderTruncated {
                location,
                remaining_bytes,
            } => write!(
                formatter,
                "MWC block header is truncated at file {} offset {}: need {BLOCK_HEADER_SIZE} bytes, found {remaining_bytes}",
                location.file_number, location.offset
            ),
            Self::PayloadEndOverflow {
                location,
                compressed_size,
            } => write!(
                formatter,
                "MWC payload end overflows this platform at file {} offset {}: compressed_size={compressed_size}",
                location.file_number, location.offset
            ),
            Self::PayloadTruncated {
                location,
                compressed_size,
                available_bytes,
            } => write!(
                formatter,
                "MWC payload is truncated at file {} offset {}: declared {compressed_size} bytes, found {available_bytes}",
                location.file_number, location.offset
            ),
        }
    }
}

impl Error for BlockScanError {}

/// Failures found while decoding a structurally valid block descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockDecodeError {
    OutputLimitExceeded {
        location: BlockLocation,
        declared_size: u32,
        max_output_size: usize,
    },
    PayloadOutOfBounds {
        location: BlockLocation,
        payload_offset: usize,
        compressed_size: u32,
        file_size: usize,
    },
    PayloadSizeMismatch {
        location: BlockLocation,
        expected_size: u32,
        actual_size: usize,
    },
    InvalidZlib {
        location: BlockLocation,
        message: String,
    },
    OutputSizeMismatch {
        location: BlockLocation,
        expected_size: u32,
        actual_size: usize,
    },
}

impl fmt::Display for BlockDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutputLimitExceeded {
                location,
                declared_size,
                max_output_size,
            } => write!(
                formatter,
                "MWC block at file {} offset {} declares {declared_size} output bytes, exceeding limit {max_output_size}",
                location.file_number, location.offset
            ),
            Self::PayloadOutOfBounds {
                location,
                payload_offset,
                compressed_size,
                file_size,
            } => write!(
                formatter,
                "MWC payload is outside file {}: block offset {}, payload offset {payload_offset}, compressed size {compressed_size}, file size {file_size}",
                location.file_number, location.offset
            ),
            Self::PayloadSizeMismatch {
                location,
                expected_size,
                actual_size,
            } => write!(
                formatter,
                "MWC payload size mismatch at file {} offset {}: expected {expected_size}, got {actual_size}",
                location.file_number, location.offset
            ),
            Self::InvalidZlib { location, message } => write!(
                formatter,
                "invalid zlib payload at file {} offset {}: {message}",
                location.file_number, location.offset
            ),
            Self::OutputSizeMismatch {
                location,
                expected_size,
                actual_size,
            } => write!(
                formatter,
                "MWC output size mismatch at file {} offset {}: expected {expected_size}, got {actual_size}",
                location.file_number, location.offset
            ),
        }
    }
}

impl Error for BlockDecodeError {}

impl MwcBlock {
    /// Decompresses a block without allowing output beyond the caller's limit.
    pub fn decode(
        &self,
        file_bytes: &[u8],
        max_output_size: usize,
    ) -> Result<Vec<u8>, BlockDecodeError> {
        let declared_size = usize::try_from(self.uncompressed_size).map_err(|_| {
            BlockDecodeError::OutputLimitExceeded {
                location: self.location,
                declared_size: self.uncompressed_size,
                max_output_size,
            }
        })?;
        if declared_size > max_output_size {
            return Err(BlockDecodeError::OutputLimitExceeded {
                location: self.location,
                declared_size: self.uncompressed_size,
                max_output_size,
            });
        }

        let compressed_size = usize::try_from(self.compressed_size).map_err(|_| {
            BlockDecodeError::PayloadOutOfBounds {
                location: self.location,
                payload_offset: self.payload_offset,
                compressed_size: self.compressed_size,
                file_size: file_bytes.len(),
            }
        })?;
        let payload_end = self.payload_offset.checked_add(compressed_size).ok_or(
            BlockDecodeError::PayloadOutOfBounds {
                location: self.location,
                payload_offset: self.payload_offset,
                compressed_size: self.compressed_size,
                file_size: file_bytes.len(),
            },
        )?;
        let payload = file_bytes.get(self.payload_offset..payload_end).ok_or(
            BlockDecodeError::PayloadOutOfBounds {
                location: self.location,
                payload_offset: self.payload_offset,
                compressed_size: self.compressed_size,
                file_size: file_bytes.len(),
            },
        )?;

        self.decode_payload_with_size(payload, declared_size)
    }

    /// Decompresses an exact compressed payload read independently from its data file.
    pub fn decode_payload(
        &self,
        payload: &[u8],
        max_output_size: usize,
    ) -> Result<Vec<u8>, BlockDecodeError> {
        let declared_size = usize::try_from(self.uncompressed_size).map_err(|_| {
            BlockDecodeError::OutputLimitExceeded {
                location: self.location,
                declared_size: self.uncompressed_size,
                max_output_size,
            }
        })?;
        if declared_size > max_output_size {
            return Err(BlockDecodeError::OutputLimitExceeded {
                location: self.location,
                declared_size: self.uncompressed_size,
                max_output_size,
            });
        }

        let compressed_size = usize::try_from(self.compressed_size).map_err(|_| {
            BlockDecodeError::PayloadSizeMismatch {
                location: self.location,
                expected_size: self.compressed_size,
                actual_size: payload.len(),
            }
        })?;
        if payload.len() != compressed_size {
            return Err(BlockDecodeError::PayloadSizeMismatch {
                location: self.location,
                expected_size: self.compressed_size,
                actual_size: payload.len(),
            });
        }

        self.decode_payload_with_size(payload, declared_size)
    }

    fn decode_payload_with_size(
        &self,
        payload: &[u8],
        declared_size: usize,
    ) -> Result<Vec<u8>, BlockDecodeError> {
        let decoder = ZlibDecoder::new(payload);
        let output_cap = u64::from(self.uncompressed_size) + 1;
        let mut limited_decoder = decoder.take(output_cap);
        let mut output = Vec::with_capacity(declared_size);
        limited_decoder.read_to_end(&mut output).map_err(|error| {
            BlockDecodeError::InvalidZlib {
                location: self.location,
                message: error.to_string(),
            }
        })?;

        if output.len() != declared_size {
            return Err(BlockDecodeError::OutputSizeMismatch {
                location: self.location,
                expected_size: self.uncompressed_size,
                actual_size: output.len(),
            });
        }

        Ok(output)
    }
}

/// Walks a complete data file without discarding bytes between MWC blocks.
pub fn scan_data_file(file_number: u32, bytes: &[u8]) -> Result<ScannedDataFile, BlockScanError> {
    let mut segments = Vec::new();
    let mut cursor = 0;

    while cursor < bytes.len() {
        let Some(relative_magic_offset) = find_magic(&bytes[cursor..]) else {
            segments.push(DataSegment::UnresolvedGap(UnresolvedGap {
                location: BlockLocation {
                    file_number,
                    offset: cursor,
                },
                len: bytes.len() - cursor,
            }));
            break;
        };
        let magic_offset = cursor + relative_magic_offset;

        if magic_offset > cursor {
            segments.push(DataSegment::UnresolvedGap(UnresolvedGap {
                location: BlockLocation {
                    file_number,
                    offset: cursor,
                },
                len: magic_offset - cursor,
            }));
        }

        let remaining_bytes = bytes.len() - magic_offset;
        let location = BlockLocation {
            file_number,
            offset: magic_offset,
        };
        if remaining_bytes < BLOCK_HEADER_SIZE {
            return Err(BlockScanError::HeaderTruncated {
                location,
                remaining_bytes,
            });
        }

        let uncompressed_size = read_u32(bytes, magic_offset + 4);
        let compressed_size = read_u32(bytes, magic_offset + 8);
        let payload_offset = magic_offset + BLOCK_HEADER_SIZE;
        let compressed_len =
            usize::try_from(compressed_size).map_err(|_| BlockScanError::PayloadEndOverflow {
                location,
                compressed_size,
            })?;
        let payload_end = payload_offset.checked_add(compressed_len).ok_or(
            BlockScanError::PayloadEndOverflow {
                location,
                compressed_size,
            },
        )?;
        if payload_end > bytes.len() {
            return Err(BlockScanError::PayloadTruncated {
                location,
                compressed_size,
                available_bytes: bytes.len() - payload_offset,
            });
        }

        segments.push(DataSegment::ZlibBlock(MwcBlock {
            location,
            uncompressed_size,
            compressed_size,
            payload_offset,
        }));
        cursor = payload_end;
    }

    Ok(ScannedDataFile {
        file_number,
        file_size: bytes.len(),
        segments,
    })
}

fn find_magic(bytes: &[u8]) -> Option<usize> {
    bytes
        .windows(MWC_MAGIC.len())
        .position(|window| window == MWC_MAGIC)
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + size_of::<u32>()]
            .try_into()
            .expect("the caller validated the MWC block header"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write;

    fn encoded_block(raw: &[u8], declared_size: u32) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(raw).expect("encode test data");
        let compressed = encoder.finish().expect("finish test data");

        let mut block = Vec::new();
        block.extend_from_slice(&MWC_MAGIC);
        block.extend_from_slice(&declared_size.to_le_bytes());
        block.extend_from_slice(
            &u32::try_from(compressed.len())
                .expect("small test payload")
                .to_le_bytes(),
        );
        block.extend_from_slice(&compressed);
        block
    }

    #[test]
    fn scans_blocks_and_preserves_every_gap_in_order() {
        let first = encoded_block(b"first", 5);
        let second = encoded_block(b"second", 6);
        let mut bytes = vec![0xAA, 0xBB];
        bytes.extend_from_slice(&first);
        bytes.extend_from_slice(&[0x10, 0x20, 0x30]);
        bytes.extend_from_slice(&second);
        bytes.push(0xCC);

        let scanned = scan_data_file(2, &bytes).expect("scan data file");

        assert_eq!(scanned.file_number, 2);
        assert_eq!(scanned.file_size, bytes.len());
        assert_eq!(scanned.segments.len(), 5);
        assert_eq!(scanned.zlib_blocks().count(), 2);
        assert_eq!(
            scanned
                .unresolved_gaps()
                .map(|gap| gap.len)
                .collect::<Vec<_>>(),
            [2, 3, 1]
        );
        assert_eq!(
            scanned
                .zlib_blocks()
                .next()
                .expect("first block")
                .decode(&bytes, 16),
            Ok(b"first".to_vec())
        );
    }

    #[test]
    fn decodes_a_payload_read_separately_from_its_data_file() {
        let bytes = encoded_block(b"separate", 8);
        let scanned = scan_data_file(4, &bytes).expect("scan data file");
        let block = scanned.zlib_blocks().next().expect("one block");
        let payload_end = block.payload_offset + block.compressed_size as usize;

        let decoded = block
            .decode_payload(&bytes[block.payload_offset..payload_end], 8)
            .expect("decode standalone payload");

        assert_eq!(decoded, b"separate");
    }

    #[test]
    fn rejects_a_standalone_payload_with_the_wrong_size() {
        let bytes = encoded_block(b"size", 4);
        let scanned = scan_data_file(5, &bytes).expect("scan data file");
        let block = scanned.zlib_blocks().next().expect("one block");

        assert_eq!(
            block.decode_payload(&[], 4),
            Err(BlockDecodeError::PayloadSizeMismatch {
                location: block.location,
                expected_size: block.compressed_size,
                actual_size: 0,
            })
        );
    }

    #[test]
    fn rejects_a_truncated_header() {
        let mut bytes = MWC_MAGIC.to_vec();
        bytes.extend_from_slice(&[0; 3]);

        assert_eq!(
            scan_data_file(1, &bytes),
            Err(BlockScanError::HeaderTruncated {
                location: BlockLocation {
                    file_number: 1,
                    offset: 0,
                },
                remaining_bytes: 7,
            })
        );
    }

    #[test]
    fn rejects_a_truncated_payload() {
        let mut bytes = MWC_MAGIC.to_vec();
        bytes.extend_from_slice(&10_u32.to_le_bytes());
        bytes.extend_from_slice(&20_u32.to_le_bytes());
        bytes.extend_from_slice(&[1, 2, 3]);

        assert_eq!(
            scan_data_file(3, &bytes),
            Err(BlockScanError::PayloadTruncated {
                location: BlockLocation {
                    file_number: 3,
                    offset: 0,
                },
                compressed_size: 20,
                available_bytes: 3,
            })
        );
    }

    #[test]
    fn enforces_the_decode_output_limit() {
        let bytes = encoded_block(b"limit", 5);
        let scanned = scan_data_file(1, &bytes).expect("scan data file");
        let block = scanned.zlib_blocks().next().expect("one block");

        assert_eq!(
            block.decode(&bytes, 4),
            Err(BlockDecodeError::OutputLimitExceeded {
                location: block.location,
                declared_size: 5,
                max_output_size: 4,
            })
        );
    }

    #[test]
    fn rejects_invalid_zlib_data() {
        let mut bytes = MWC_MAGIC.to_vec();
        bytes.extend_from_slice(&4_u32.to_le_bytes());
        bytes.extend_from_slice(&4_u32.to_le_bytes());
        bytes.extend_from_slice(b"nope");
        let scanned = scan_data_file(1, &bytes).expect("structurally valid block");
        let block = scanned.zlib_blocks().next().expect("one block");

        assert!(matches!(
            block.decode(&bytes, 8),
            Err(BlockDecodeError::InvalidZlib { .. })
        ));
    }

    #[test]
    fn rejects_a_decompressed_size_mismatch() {
        let bytes = encoded_block(b"four", 5);
        let scanned = scan_data_file(1, &bytes).expect("scan data file");
        let block = scanned.zlib_blocks().next().expect("one block");

        assert_eq!(
            block.decode(&bytes, 8),
            Err(BlockDecodeError::OutputSizeMismatch {
                location: block.location,
                expected_size: 5,
                actual_size: 4,
            })
        );
    }
}
