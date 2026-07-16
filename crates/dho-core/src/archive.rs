// SPDX-License-Identifier: MPL-2.0

use crate::{
    BlockDecodeError, BlockLocation, DataSegment, IndexedArchive, MwcBlock, ScannedDataFile,
    UnresolvedGap,
};
use std::borrow::Cow;
use std::collections::HashSet;
use std::error::Error;
use std::fmt;

/// A block whose position in the archive stream has been established.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveBlock {
    /// The global index is absent when unresolved data prevents a reliable ordering.
    pub block_index: Option<u32>,
    pub kind: ArchiveBlockKind,
}

/// A headerless block whose resource meaning was established by cross-validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawBlock {
    location: BlockLocation,
    len: usize,
}

impl RawBlock {
    fn from_gap(gap: UnresolvedGap) -> Self {
        Self {
            location: gap.location,
            len: gap.len,
        }
    }

    pub fn location(self) -> BlockLocation {
        self.location
    }

    pub fn len(self) -> usize {
        self.len
    }

    pub fn is_empty(self) -> bool {
        self.len == 0
    }
}

/// The storage form of one logical image block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveBlockKind {
    Zlib(MwcBlock),
    Raw(RawBlock),
}

impl ArchiveBlockKind {
    /// Byte position of this block in its numbered data file.
    pub fn location(self) -> BlockLocation {
        match self {
            Self::Zlib(block) => block.location,
            Self::Raw(block) => block.location,
        }
    }

    /// Number of bytes made available after decoding the block.
    pub fn uncompressed_size(self) -> u64 {
        match self {
            Self::Zlib(block) => u64::from(block.uncompressed_size),
            Self::Raw(block) => block.len as u64,
        }
    }

    /// Byte offset where the stored payload for this block begins.
    pub fn stored_offset(self) -> usize {
        match self {
            Self::Zlib(block) => block.payload_offset,
            Self::Raw(block) => block.location.offset,
        }
    }

    /// Number of stored bytes that must be read before decoding this block.
    pub fn stored_size(self) -> usize {
        match self {
            Self::Zlib(block) => block.compressed_size as usize,
            Self::Raw(block) => block.len,
        }
    }

    /// Decodes only the exact stored range previously read for this block.
    pub fn decode_stored(
        self,
        stored_bytes: &[u8],
        max_output_size: usize,
    ) -> Result<Vec<u8>, ArchiveBlockDecodeError> {
        match self {
            Self::Zlib(block) => block
                .decode_payload(stored_bytes, max_output_size)
                .map_err(ArchiveBlockDecodeError::Zlib),
            Self::Raw(block) => {
                if block.len > max_output_size {
                    return Err(ArchiveBlockDecodeError::OutputLimitExceeded {
                        location: block.location,
                        block_size: block.len,
                        max_output_size,
                    });
                }
                if stored_bytes.len() != block.len {
                    return Err(ArchiveBlockDecodeError::StoredSizeMismatch {
                        location: block.location,
                        expected_size: block.len,
                        actual_size: stored_bytes.len(),
                    });
                }
                Ok(stored_bytes.to_vec())
            }
        }
    }

    /// Returns decoded bytes with one output limit for compressed and raw storage.
    pub fn decode(
        self,
        file_bytes: &[u8],
        max_output_size: usize,
    ) -> Result<Cow<'_, [u8]>, ArchiveBlockDecodeError> {
        match self {
            Self::Zlib(block) => block
                .decode(file_bytes, max_output_size)
                .map(Cow::Owned)
                .map_err(ArchiveBlockDecodeError::Zlib),
            Self::Raw(block) => {
                if block.len > max_output_size {
                    return Err(ArchiveBlockDecodeError::OutputLimitExceeded {
                        location: block.location,
                        block_size: block.len,
                        max_output_size,
                    });
                }
                let end = block.location.offset.checked_add(block.len).ok_or(
                    ArchiveBlockDecodeError::RawDataEndOverflow {
                        location: block.location,
                        block_size: block.len,
                    },
                )?;
                let bytes = file_bytes.get(block.location.offset..end).ok_or(
                    ArchiveBlockDecodeError::RawDataOutOfBounds {
                        location: block.location,
                        block_size: block.len,
                        file_size: file_bytes.len(),
                    },
                )?;
                Ok(Cow::Borrowed(bytes))
            }
        }
    }
}

/// Failures found while obtaining bytes from a resolved archive block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchiveBlockDecodeError {
    Zlib(BlockDecodeError),
    OutputLimitExceeded {
        location: BlockLocation,
        block_size: usize,
        max_output_size: usize,
    },
    RawDataEndOverflow {
        location: BlockLocation,
        block_size: usize,
    },
    RawDataOutOfBounds {
        location: BlockLocation,
        block_size: usize,
        file_size: usize,
    },
    StoredSizeMismatch {
        location: BlockLocation,
        expected_size: usize,
        actual_size: usize,
    },
}

impl fmt::Display for ArchiveBlockDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zlib(error) => error.fmt(formatter),
            Self::OutputLimitExceeded {
                location,
                block_size,
                max_output_size,
            } => write!(
                formatter,
                "raw block at file {} offset {} contains {block_size} bytes, exceeding limit {max_output_size}",
                location.file_number, location.offset
            ),
            Self::RawDataEndOverflow {
                location,
                block_size,
            } => write!(
                formatter,
                "raw block end overflows this platform at file {} offset {}: block size {block_size}",
                location.file_number, location.offset
            ),
            Self::RawDataOutOfBounds {
                location,
                block_size,
                file_size,
            } => write!(
                formatter,
                "raw block is outside file {}: offset {}, block size {block_size}, file size {file_size}",
                location.file_number, location.offset
            ),
            Self::StoredSizeMismatch {
                location,
                expected_size,
                actual_size,
            } => write!(
                formatter,
                "stored block size mismatch at file {} offset {}: expected {expected_size}, got {actual_size}",
                location.file_number, location.offset
            ),
        }
    }
}

impl Error for ArchiveBlockDecodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Zlib(error) => Some(error),
            Self::OutputLimitExceeded { .. }
            | Self::RawDataEndOverflow { .. }
            | Self::RawDataOutOfBounds { .. }
            | Self::StoredSizeMismatch { .. } => None,
        }
    }
}

/// Cross-validated archive structure without ownership of the game file bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveLayout {
    pub blocks: Vec<ArchiveBlock>,
    pub unresolved_gaps: Vec<UnresolvedGap>,
    pub diagnostics: Vec<ArchiveDiagnostic>,
    block_order_resolved: bool,
}

impl ArchiveLayout {
    /// Whether every returned block has a reliable global index.
    pub fn has_resolved_block_order(&self) -> bool {
        self.block_order_resolved
    }
}

/// Facts that disagree while connecting an index to its data files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveDiagnostic {
    ArchiveFileCountMismatch {
        expected: u32,
        actual: usize,
    },
    DataFileNumberMismatch {
        position: usize,
        expected: u32,
        actual: u32,
    },
    UnexpectedDataFile {
        file_number: u32,
    },
    GroupCountMismatch {
        expected: u32,
        actual: usize,
    },
    LogicalBlockCountMismatch {
        expected: u32,
        zlib_blocks: usize,
        unresolved_gaps: usize,
    },
    RawGapNotReferenced {
        gap: UnresolvedGap,
        candidate_block_index: u32,
    },
    RawGapCandidateInconsistent {
        gap: UnresolvedGap,
        candidate_block_index: u32,
    },
    RecordBlockIndexOutOfRange {
        record_position: usize,
        block_index: u32,
        block_count: u32,
    },
    ImageByteSizeOverflow {
        record_position: usize,
        width: u32,
        height: u32,
    },
    ImageSizeMismatch {
        record_position: usize,
        block_index: u32,
        expected_size: u64,
        actual_size: u64,
    },
}

/// Connects parsed records to ordered data segments without reading or changing game files.
pub fn build_archive_layout(
    index: &IndexedArchive,
    data_files: &[ScannedDataFile],
) -> ArchiveLayout {
    let mut diagnostics = Vec::new();
    validate_group_count(index, &mut diagnostics);

    let mut main_files = data_files
        .iter()
        .filter(|file| {
            let is_main_file =
                file.file_number > 0 && file.file_number <= index.header.archive_count;
            if !is_main_file {
                diagnostics.push(ArchiveDiagnostic::UnexpectedDataFile {
                    file_number: file.file_number,
                });
            }
            is_main_file
        })
        .collect::<Vec<_>>();
    main_files.sort_unstable_by_key(|file| file.file_number);

    let expected_file_count = index.header.archive_count;
    let mut file_set_is_complete = main_files.len() as u64 == u64::from(expected_file_count);
    if !file_set_is_complete {
        diagnostics.push(ArchiveDiagnostic::ArchiveFileCountMismatch {
            expected: expected_file_count,
            actual: main_files.len(),
        });
    }

    for (position, file) in main_files.iter().enumerate() {
        let expected_number = u32::try_from(position + 1).unwrap_or(u32::MAX);
        if file.file_number != expected_number {
            file_set_is_complete = false;
            diagnostics.push(ArchiveDiagnostic::DataFileNumberMismatch {
                position,
                expected: expected_number,
                actual: file.file_number,
            });
        }
    }

    let segments = main_files
        .iter()
        .flat_map(|file| file.segments.iter().copied())
        .collect::<Vec<_>>();
    let zlib_blocks = segments
        .iter()
        .filter_map(|segment| match segment {
            DataSegment::ZlibBlock(block) => Some(*block),
            DataSegment::UnresolvedGap(_) => None,
        })
        .collect::<Vec<_>>();
    let gaps = segments
        .iter()
        .filter_map(|segment| match segment {
            DataSegment::ZlibBlock(_) => None,
            DataSegment::UnresolvedGap(gap) => Some(*gap),
        })
        .collect::<Vec<_>>();

    let expected_blocks = u64::from(index.header.image_block_count);
    let resolved_kinds = if file_set_is_complete && zlib_blocks.len() as u64 == expected_blocks {
        Some(
            zlib_blocks
                .iter()
                .copied()
                .map(ArchiveBlockKind::Zlib)
                .collect::<Vec<_>>(),
        )
    } else if file_set_is_complete
        && gaps.len() == 1
        && zlib_blocks.len() as u64 + 1 == expected_blocks
    {
        resolve_single_raw_gap(index, &segments, gaps[0], &mut diagnostics)
    } else {
        None
    };

    let block_order_resolved = resolved_kinds.is_some();
    let (blocks, unresolved_gaps) = if let Some(kinds) = resolved_kinds {
        let raw_gap = kinds.iter().find_map(|kind| match kind {
            ArchiveBlockKind::Zlib(_) => None,
            ArchiveBlockKind::Raw(block) => Some(UnresolvedGap {
                location: block.location,
                len: block.len,
            }),
        });
        let blocks = kinds
            .into_iter()
            .enumerate()
            .map(|(position, kind)| ArchiveBlock {
                block_index: Some(
                    u32::try_from(position)
                        .expect("resolved block count is bounded by the u32 archive header"),
                ),
                kind,
            })
            .collect::<Vec<_>>();
        let unresolved_gaps = gaps
            .iter()
            .copied()
            .filter(|gap| Some(*gap) != raw_gap)
            .collect::<Vec<_>>();
        validate_records(index, &blocks, &mut diagnostics);
        (blocks, unresolved_gaps)
    } else {
        if zlib_blocks.len() as u64 != expected_blocks {
            diagnostics.push(ArchiveDiagnostic::LogicalBlockCountMismatch {
                expected: index.header.image_block_count,
                zlib_blocks: zlib_blocks.len(),
                unresolved_gaps: gaps.len(),
            });
        }
        validate_record_ranges(index, &mut diagnostics);
        (
            zlib_blocks
                .into_iter()
                .map(|block| ArchiveBlock {
                    block_index: None,
                    kind: ArchiveBlockKind::Zlib(block),
                })
                .collect(),
            gaps,
        )
    };

    ArchiveLayout {
        blocks,
        unresolved_gaps,
        diagnostics,
        block_order_resolved,
    }
}

fn resolve_single_raw_gap(
    index: &IndexedArchive,
    segments: &[DataSegment],
    gap: UnresolvedGap,
    diagnostics: &mut Vec<ArchiveDiagnostic>,
) -> Option<Vec<ArchiveBlockKind>> {
    let kinds = segments
        .iter()
        .map(|segment| match segment {
            DataSegment::ZlibBlock(block) => ArchiveBlockKind::Zlib(*block),
            DataSegment::UnresolvedGap(gap) => ArchiveBlockKind::Raw(RawBlock::from_gap(*gap)),
        })
        .collect::<Vec<_>>();
    let gap_position = kinds
        .iter()
        .position(|kind| matches!(kind, ArchiveBlockKind::Raw(_)))
        .expect("the caller supplied the only unresolved gap");
    let candidate_block_index =
        u32::try_from(gap_position).expect("candidate count is bounded by the u32 archive header");
    let is_referenced = index
        .records
        .iter()
        .any(|record| record.block_index == candidate_block_index);

    if !is_referenced {
        diagnostics.push(ArchiveDiagnostic::RawGapNotReferenced {
            gap,
            candidate_block_index,
        });
        return None;
    }

    if !records_match_candidate(index, &kinds) {
        diagnostics.push(ArchiveDiagnostic::RawGapCandidateInconsistent {
            gap,
            candidate_block_index,
        });
        return None;
    }

    Some(kinds)
}

fn records_match_candidate(index: &IndexedArchive, kinds: &[ArchiveBlockKind]) -> bool {
    index.records.iter().all(|record| {
        let Ok(block_index) = usize::try_from(record.block_index) else {
            return false;
        };
        let Some(kind) = kinds.get(block_index) else {
            return false;
        };
        let Some(expected_size) = image_byte_size(record.width, record.height) else {
            return false;
        };
        expected_size == kind.uncompressed_size()
    })
}

fn validate_group_count(index: &IndexedArchive, diagnostics: &mut Vec<ArchiveDiagnostic>) {
    let actual = index
        .records
        .iter()
        .map(|record| record.group_code)
        .collect::<HashSet<_>>()
        .len();
    if actual as u64 != u64::from(index.header.group_count) {
        diagnostics.push(ArchiveDiagnostic::GroupCountMismatch {
            expected: index.header.group_count,
            actual,
        });
    }
}

fn validate_record_ranges(index: &IndexedArchive, diagnostics: &mut Vec<ArchiveDiagnostic>) {
    for (record_position, record) in index.records.iter().enumerate() {
        if record.block_index >= index.header.image_block_count {
            diagnostics.push(ArchiveDiagnostic::RecordBlockIndexOutOfRange {
                record_position,
                block_index: record.block_index,
                block_count: index.header.image_block_count,
            });
        }
        if image_byte_size(record.width, record.height).is_none() {
            diagnostics.push(ArchiveDiagnostic::ImageByteSizeOverflow {
                record_position,
                width: record.width,
                height: record.height,
            });
        }
    }
}

fn validate_records(
    index: &IndexedArchive,
    blocks: &[ArchiveBlock],
    diagnostics: &mut Vec<ArchiveDiagnostic>,
) {
    for (record_position, record) in index.records.iter().enumerate() {
        let Ok(block_index) = usize::try_from(record.block_index) else {
            diagnostics.push(ArchiveDiagnostic::RecordBlockIndexOutOfRange {
                record_position,
                block_index: record.block_index,
                block_count: index.header.image_block_count,
            });
            continue;
        };
        let Some(block) = blocks.get(block_index) else {
            diagnostics.push(ArchiveDiagnostic::RecordBlockIndexOutOfRange {
                record_position,
                block_index: record.block_index,
                block_count: index.header.image_block_count,
            });
            continue;
        };
        let Some(expected_size) = image_byte_size(record.width, record.height) else {
            diagnostics.push(ArchiveDiagnostic::ImageByteSizeOverflow {
                record_position,
                width: record.width,
                height: record.height,
            });
            continue;
        };
        let actual_size = block.kind.uncompressed_size();
        if expected_size != actual_size {
            diagnostics.push(ArchiveDiagnostic::ImageSizeMismatch {
                record_position,
                block_index: record.block_index,
                expected_size,
                actual_size,
            });
        }
    }
}

fn image_byte_size(width: u32, height: u32) -> Option<u64> {
    u64::from(width)
        .checked_mul(u64::from(height))?
        .checked_mul(4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArchiveHeader, BlockLocation, IndexRecord};
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write;

    fn index(
        group_count: u32,
        image_block_count: u32,
        archive_count: u32,
        records: Vec<IndexRecord>,
    ) -> IndexedArchive {
        IndexedArchive {
            header: ArchiveHeader {
                record_count: records.len() as u32,
                group_count,
                default_width: 1,
                default_height: 1,
                image_block_count,
                archive_count,
                reserved: 0,
            },
            records,
            trailing_index_bytes: 0,
        }
    }

    fn record(block_index: u32, width: u32, height: u32, group_code: u32) -> IndexRecord {
        IndexRecord {
            icon_id: block_index + 100,
            block_index,
            width,
            height,
            group_code,
        }
    }

    fn zlib(file_number: u32, offset: usize, uncompressed_size: u32) -> DataSegment {
        DataSegment::ZlibBlock(MwcBlock {
            location: BlockLocation {
                file_number,
                offset,
            },
            uncompressed_size,
            compressed_size: 1,
            payload_offset: offset + 12,
        })
    }

    fn gap(file_number: u32, offset: usize, len: usize) -> DataSegment {
        DataSegment::UnresolvedGap(UnresolvedGap {
            location: BlockLocation {
                file_number,
                offset,
            },
            len,
        })
    }

    fn file(file_number: u32, segments: Vec<DataSegment>) -> ScannedDataFile {
        ScannedDataFile {
            file_number,
            file_size: 100,
            segments,
        }
    }

    #[test]
    fn assigns_global_indices_across_data_files() {
        let index = index(1, 2, 2, vec![record(0, 1, 1, 9), record(1, 1, 2, 9)]);
        let files = [file(1, vec![zlib(1, 0, 4)]), file(2, vec![zlib(2, 0, 8)])];

        let layout = build_archive_layout(&index, &files);

        assert!(layout.has_resolved_block_order());
        assert_eq!(
            layout
                .blocks
                .iter()
                .map(|block| block.block_index)
                .collect::<Vec<_>>(),
            [Some(0), Some(1)]
        );
        assert!(layout.unresolved_gaps.is_empty());
        assert!(layout.diagnostics.is_empty());
    }

    #[test]
    fn promotes_one_fully_consistent_raw_gap() {
        let index = index(
            1,
            3,
            1,
            vec![record(0, 1, 1, 9), record(1, 1, 2, 9), record(2, 1, 3, 9)],
        );
        let raw_gap = gap(1, 20, 8);
        let files = [file(1, vec![zlib(1, 0, 4), raw_gap, zlib(1, 28, 12)])];

        let layout = build_archive_layout(&index, &files);

        assert!(layout.has_resolved_block_order());
        assert!(matches!(layout.blocks[1].kind, ArchiveBlockKind::Raw(_)));
        assert!(layout.unresolved_gaps.is_empty());
        assert!(layout.diagnostics.is_empty());
    }

    #[test]
    fn leaves_an_unreferenced_gap_unresolved() {
        let index = index(1, 2, 1, vec![record(0, 1, 1, 9)]);
        let raw_gap = gap(1, 20, 8);
        let files = [file(1, vec![zlib(1, 0, 4), raw_gap])];

        let layout = build_archive_layout(&index, &files);

        assert!(!layout.has_resolved_block_order());
        assert_eq!(layout.unresolved_gaps.len(), 1);
        assert!(
            layout
                .diagnostics
                .contains(&ArchiveDiagnostic::RawGapNotReferenced {
                    gap: match raw_gap {
                        DataSegment::UnresolvedGap(gap) => gap,
                        DataSegment::ZlibBlock(_) => unreachable!(),
                    },
                    candidate_block_index: 1,
                })
        );
    }

    #[test]
    fn rejects_a_raw_gap_when_later_records_do_not_match() {
        let index = index(1, 3, 1, vec![record(1, 1, 2, 9), record(2, 1, 4, 9)]);
        let raw_gap = gap(1, 20, 8);
        let files = [file(1, vec![zlib(1, 0, 4), raw_gap, zlib(1, 28, 12)])];

        let layout = build_archive_layout(&index, &files);

        assert!(!layout.has_resolved_block_order());
        assert_eq!(layout.unresolved_gaps.len(), 1);
        assert!(matches!(
            layout.diagnostics.first(),
            Some(ArchiveDiagnostic::RawGapCandidateInconsistent { .. })
        ));
    }

    #[test]
    fn ignores_files_beyond_archive_count() {
        let index = index(1, 1, 1, vec![record(0, 1, 1, 9)]);
        let files = [file(1, vec![zlib(1, 0, 4)]), file(2, vec![zlib(2, 0, 4)])];

        let layout = build_archive_layout(&index, &files);

        assert!(layout.has_resolved_block_order());
        assert_eq!(layout.blocks.len(), 1);
        assert_eq!(
            layout.diagnostics,
            [ArchiveDiagnostic::UnexpectedDataFile { file_number: 2 }]
        );
    }

    #[test]
    fn reports_header_record_and_size_inconsistencies() {
        let index = index(2, 1, 1, vec![record(0, 1, 2, 9), record(1, 1, 1, 9)]);
        let files = [file(1, vec![zlib(1, 0, 4)])];

        let layout = build_archive_layout(&index, &files);

        assert!(
            layout
                .diagnostics
                .contains(&ArchiveDiagnostic::GroupCountMismatch {
                    expected: 2,
                    actual: 1,
                })
        );
        assert!(
            layout
                .diagnostics
                .contains(&ArchiveDiagnostic::ImageSizeMismatch {
                    record_position: 0,
                    block_index: 0,
                    expected_size: 8,
                    actual_size: 4,
                })
        );
        assert!(
            layout
                .diagnostics
                .contains(&ArchiveDiagnostic::RecordBlockIndexOutOfRange {
                    record_position: 1,
                    block_index: 1,
                    block_count: 1,
                })
        );
    }

    #[test]
    fn decodes_zlib_storage_to_owned_bytes() {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"data").expect("write zlib input");
        let compressed = encoder.finish().expect("finish zlib stream");
        let kind = ArchiveBlockKind::Zlib(MwcBlock {
            location: BlockLocation {
                file_number: 1,
                offset: 0,
            },
            uncompressed_size: 4,
            compressed_size: compressed.len() as u32,
            payload_offset: 0,
        });

        let decoded = kind.decode(&compressed, 4).expect("decode zlib block");

        assert!(matches!(decoded, Cow::Owned(bytes) if bytes == b"data"));
    }

    #[test]
    fn decodes_only_the_stored_zlib_payload() {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"range").expect("write zlib input");
        let compressed = encoder.finish().expect("finish zlib stream");
        let kind = ArchiveBlockKind::Zlib(MwcBlock {
            location: BlockLocation {
                file_number: 3,
                offset: 40,
            },
            uncompressed_size: 5,
            compressed_size: compressed.len() as u32,
            payload_offset: 52,
        });

        let decoded = kind
            .decode_stored(&compressed, 5)
            .expect("decode stored payload");

        assert_eq!(kind.stored_offset(), 52);
        assert_eq!(kind.stored_size(), compressed.len());
        assert_eq!(decoded, b"range");
    }

    #[test]
    fn reads_raw_storage_without_copying() {
        let kind = ArchiveBlockKind::Raw(RawBlock {
            location: BlockLocation {
                file_number: 2,
                offset: 1,
            },
            len: 2,
        });

        let decoded = kind
            .decode(&[0x00, 0x11, 0x22, 0x33], 2)
            .expect("read raw block");

        assert!(matches!(decoded, Cow::Borrowed(bytes) if bytes == [0x11, 0x22]));
    }

    #[test]
    fn decodes_only_the_stored_raw_range() {
        let kind = ArchiveBlockKind::Raw(RawBlock {
            location: BlockLocation {
                file_number: 2,
                offset: 7,
            },
            len: 3,
        });

        let decoded = kind
            .decode_stored(&[0x11, 0x22, 0x33], 3)
            .expect("decode stored raw range");

        assert_eq!(kind.stored_offset(), 7);
        assert_eq!(kind.stored_size(), 3);
        assert_eq!(decoded, [0x11, 0x22, 0x33]);
    }

    #[test]
    fn rejects_a_stored_raw_range_with_the_wrong_size() {
        let kind = ArchiveBlockKind::Raw(RawBlock {
            location: BlockLocation {
                file_number: 2,
                offset: 7,
            },
            len: 3,
        });

        assert_eq!(
            kind.decode_stored(&[0x11], 3),
            Err(ArchiveBlockDecodeError::StoredSizeMismatch {
                location: kind.location(),
                expected_size: 3,
                actual_size: 1,
            })
        );
    }

    #[test]
    fn enforces_the_output_limit_for_raw_storage() {
        let kind = ArchiveBlockKind::Raw(RawBlock {
            location: BlockLocation {
                file_number: 2,
                offset: 0,
            },
            len: 4,
        });

        assert_eq!(
            kind.decode(&[0; 4], 3),
            Err(ArchiveBlockDecodeError::OutputLimitExceeded {
                location: kind.location(),
                block_size: 4,
                max_output_size: 3,
            })
        );
    }

    #[test]
    fn rejects_raw_storage_outside_the_file() {
        let kind = ArchiveBlockKind::Raw(RawBlock {
            location: BlockLocation {
                file_number: 2,
                offset: 2,
            },
            len: 3,
        });

        assert_eq!(
            kind.decode(&[0; 4], 3),
            Err(ArchiveBlockDecodeError::RawDataOutOfBounds {
                location: kind.location(),
                block_size: 3,
                file_size: 4,
            })
        );
    }

    #[test]
    fn rejects_a_raw_end_offset_overflow() {
        let kind = ArchiveBlockKind::Raw(RawBlock {
            location: BlockLocation {
                file_number: 2,
                offset: usize::MAX,
            },
            len: 1,
        });

        assert_eq!(
            kind.decode(&[], 1),
            Err(ArchiveBlockDecodeError::RawDataEndOverflow {
                location: kind.location(),
                block_size: 1,
            })
        );
    }

    #[test]
    fn does_not_report_a_missing_archive_as_resolved() {
        let index = index(0, 0, 1, Vec::new());

        let layout = build_archive_layout(&index, &[]);

        assert!(!layout.has_resolved_block_order());
        assert_eq!(
            layout.diagnostics,
            [ArchiveDiagnostic::ArchiveFileCountMismatch {
                expected: 1,
                actual: 0,
            }]
        );
    }
}
