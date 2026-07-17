// SPDX-License-Identifier: MPL-2.0

//! Read-only loading and on-demand extraction of indexed DHO image resources.

use dho_catalog::{AssemblyPlan, VerificationStatus, assembly_plan};
use dho_core::{
    ArchiveBlockDecodeError, ArchiveDiagnostic, ArchiveLayout, BlockScanError, IndexParseError,
    IndexRecord, IndexedArchive, build_archive_layout, scan_data_file,
};
use dho_image::{ImageAssemblyError, PixelDecodeError, PngEncodeError, RgbaImage};
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// A validated two-letter filename prefix such as `sc` or `sd`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArchivePrefix(String);

impl ArchivePrefix {
    pub fn parse(value: &str) -> Result<Self, ArchivePrefixError> {
        if value.len() != 2 || !value.bytes().all(|byte| byte.is_ascii_alphabetic()) {
            return Err(ArchivePrefixError {
                provided: value.to_owned(),
            });
        }
        Ok(Self(value.to_ascii_lowercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A physical resource key that preserves distinct block variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceKey {
    pub group_code: u32,
    pub icon_id: u32,
    pub block_index: u32,
}

/// One requested resource encoded for display or download.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedResource {
    pub key: ResourceKey,
    pub width: u32,
    pub height: u32,
    pub png: Vec<u8>,
}

/// One completed image joined from a human-verified physical block range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedAssembly {
    pub first_block: u32,
    pub last_block: u32,
    pub width: u32,
    pub height: u32,
    pub png: Vec<u8>,
}

#[derive(Debug)]
struct ArchiveDataFile {
    file_number: u32,
    path: PathBuf,
}

/// A single archive family loaded once, with images decoded only when requested.
#[derive(Debug)]
pub struct LoadedArchive {
    prefix: ArchivePrefix,
    index: IndexedArchive,
    layout: ArchiveLayout,
    data_files: Vec<ArchiveDataFile>,
}

impl LoadedArchive {
    /// Opens the index and only the numbered data files declared by its header.
    pub fn open(directory: impl AsRef<Path>, prefix: &str) -> Result<Self, ExtractError> {
        let directory = directory.as_ref();
        let prefix = ArchivePrefix::parse(prefix).map_err(ExtractError::InvalidPrefix)?;
        let index_path = directory.join(format!("{}000000.bin", prefix.as_str()));
        let index_bytes = read_file("read archive index", &index_path)?;
        let index = IndexedArchive::parse(&index_bytes).map_err(ExtractError::IndexParse)?;

        let mut scanned_files = Vec::new();
        let mut data_files = Vec::new();
        for file_number in 1..=index.header.archive_count {
            let path = directory.join(format!("{}{file_number:06}.bin", prefix.as_str()));
            let bytes = read_file("read archive data", &path)?;
            let scanned =
                scan_data_file(file_number, &bytes).map_err(|source| ExtractError::BlockScan {
                    file_number,
                    source,
                })?;
            scanned_files.push(scanned);
            data_files.push(ArchiveDataFile { file_number, path });
        }

        let layout = build_archive_layout(&index, &scanned_files);
        if !layout.has_resolved_block_order() || !layout.diagnostics.is_empty() {
            return Err(ExtractError::InvalidLayout {
                diagnostics: layout.diagnostics,
            });
        }

        Ok(Self {
            prefix,
            index,
            layout,
            data_files,
        })
    }

    pub fn prefix(&self) -> &ArchivePrefix {
        &self.prefix
    }

    pub fn records(&self) -> &[IndexRecord] {
        &self.index.records
    }

    pub fn resource_keys(&self) -> impl Iterator<Item = ResourceKey> + '_ {
        self.index.records.iter().map(|record| ResourceKey {
            group_code: record.group_code,
            icon_id: record.icon_id,
            block_index: record.block_index,
        })
    }

    /// Decodes and PNG-encodes exactly one physical record variant.
    pub fn extract_png(
        &self,
        key: ResourceKey,
        max_output_size: usize,
    ) -> Result<ExtractedResource, ExtractError> {
        let mut matching_records = self.index.records.iter().filter(|record| {
            record.group_code == key.group_code
                && record.icon_id == key.icon_id
                && record.block_index == key.block_index
        });
        let record = matching_records
            .next()
            .copied()
            .ok_or(ExtractError::ResourceNotFound { key })?;
        if let Some(conflict) = matching_records
            .copied()
            .find(|candidate| candidate.width != record.width || candidate.height != record.height)
        {
            return Err(ExtractError::ConflictingRecordDimensions {
                key,
                first: (record.width, record.height),
                conflicting: (conflict.width, conflict.height),
            });
        }

        let image = self.decode_record(record, max_output_size)?;
        let png = image.encode_png().map_err(ExtractError::PngEncode)?;

        Ok(ExtractedResource {
            key,
            width: record.width,
            height: record.height,
            png,
        })
    }

    /// Joins the completed image containing a block only when its rule was human-verified.
    pub fn extract_verified_assembly(
        &self,
        block_index: u32,
        max_tile_output_size: usize,
        max_assembled_output_size: usize,
    ) -> Result<Option<ExtractedAssembly>, ExtractError> {
        let Some(plan) = assembly_plan(self.prefix.as_str(), block_index) else {
            return Ok(None);
        };

        self.extract_assembly(plan, max_tile_output_size, max_assembled_output_size)
            .map(Some)
    }

    fn extract_assembly(
        &self,
        plan: AssemblyPlan,
        max_tile_output_size: usize,
        max_assembled_output_size: usize,
    ) -> Result<ExtractedAssembly, ExtractError> {
        if !plan.rule.archive.eq_ignore_ascii_case(self.prefix.as_str()) {
            return Err(ExtractError::AssemblyArchiveMismatch {
                expected: plan.rule.archive,
                actual: self.prefix.as_str().to_owned(),
            });
        }
        if plan.rule.status != VerificationStatus::HumanVerified {
            return Err(ExtractError::AssemblyRuleNotVerified {
                first_block: plan.first_block,
                last_block: plan.last_block,
            });
        }

        let mut tiles = Vec::new();
        for block_index in plan.first_block..=plan.last_block {
            let record = self.record_for_block(block_index)?;
            tiles.push(self.decode_record(record, max_tile_output_size)?);
        }

        let image = RgbaImage::assemble_grid(
            &tiles,
            plan.rule.columns,
            plan.rule.rows,
            plan.rule.output_width,
            plan.rule.output_height,
            max_assembled_output_size,
        )
        .map_err(ExtractError::ImageAssembly)?;
        let png = image.encode_png().map_err(ExtractError::PngEncode)?;

        Ok(ExtractedAssembly {
            first_block: plan.first_block,
            last_block: plan.last_block,
            width: image.width(),
            height: image.height(),
            png,
        })
    }

    fn record_for_block(&self, block_index: u32) -> Result<IndexRecord, ExtractError> {
        let mut records = self
            .index
            .records
            .iter()
            .filter(|record| record.block_index == block_index)
            .copied();
        let record = records
            .next()
            .ok_or(ExtractError::AssemblyTileRecordNotFound { block_index })?;
        if let Some(conflict) = records
            .find(|candidate| candidate.width != record.width || candidate.height != record.height)
        {
            return Err(ExtractError::ConflictingBlockDimensions {
                block_index,
                first: (record.width, record.height),
                conflicting: (conflict.width, conflict.height),
            });
        }
        Ok(record)
    }

    fn decode_record(
        &self,
        record: IndexRecord,
        max_output_size: usize,
    ) -> Result<RgbaImage, ExtractError> {
        let block_index = record.block_index;
        let block = self
            .layout
            .blocks
            .get(usize::try_from(block_index).unwrap_or(usize::MAX))
            .filter(|block| block.block_index == Some(block_index))
            .ok_or(ExtractError::BlockUnavailable { block_index })?;
        let location = block.kind.location();
        let data_file = self
            .data_files
            .iter()
            .find(|file| file.file_number == location.file_number)
            .ok_or(ExtractError::DataFileUnavailable {
                file_number: location.file_number,
            })?;
        let stored_bytes = read_file_range(
            "read archive block",
            &data_file.path,
            block.kind.stored_offset(),
            block.kind.stored_size(),
        )?;
        let decoded = block
            .kind
            .decode_stored(&stored_bytes, max_output_size)
            .map_err(ExtractError::BlockDecode)?;
        let image = RgbaImage::from_bgra(record.width, record.height, &decoded)
            .map_err(ExtractError::PixelDecode)?;
        Ok(image)
    }
}

fn read_file(operation: &'static str, path: &Path) -> Result<Vec<u8>, ExtractError> {
    fs::read(path).map_err(|source| ExtractError::Io {
        operation,
        path: path.to_owned(),
        source,
    })
}

fn read_file_range(
    operation: &'static str,
    path: &Path,
    offset: usize,
    len: usize,
) -> Result<Vec<u8>, ExtractError> {
    let mut file = File::open(path).map_err(|source| ExtractError::Io {
        operation,
        path: path.to_owned(),
        source,
    })?;
    let offset = u64::try_from(offset).map_err(|_| ExtractError::Io {
        operation,
        path: path.to_owned(),
        source: io::Error::new(io::ErrorKind::InvalidInput, "block offset exceeds u64"),
    })?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|source| ExtractError::Io {
            operation,
            path: path.to_owned(),
            source,
        })?;
    let mut bytes = vec![0; len];
    file.read_exact(&mut bytes)
        .map_err(|source| ExtractError::Io {
            operation,
            path: path.to_owned(),
            source,
        })?;
    Ok(bytes)
}

/// Invalid archive filename prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivePrefixError {
    provided: String,
}

impl fmt::Display for ArchivePrefixError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "archive prefix must contain exactly two ASCII letters: {:?}",
            self.provided
        )
    }
}

impl Error for ArchivePrefixError {}

/// Failures while loading an archive or extracting one requested resource.
#[derive(Debug)]
pub enum ExtractError {
    InvalidPrefix(ArchivePrefixError),
    Io {
        operation: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    IndexParse(IndexParseError),
    BlockScan {
        file_number: u32,
        source: BlockScanError,
    },
    InvalidLayout {
        diagnostics: Vec<ArchiveDiagnostic>,
    },
    ResourceNotFound {
        key: ResourceKey,
    },
    ConflictingRecordDimensions {
        key: ResourceKey,
        first: (u32, u32),
        conflicting: (u32, u32),
    },
    AssemblyTileRecordNotFound {
        block_index: u32,
    },
    ConflictingBlockDimensions {
        block_index: u32,
        first: (u32, u32),
        conflicting: (u32, u32),
    },
    AssemblyArchiveMismatch {
        expected: &'static str,
        actual: String,
    },
    AssemblyRuleNotVerified {
        first_block: u32,
        last_block: u32,
    },
    BlockUnavailable {
        block_index: u32,
    },
    DataFileUnavailable {
        file_number: u32,
    },
    BlockDecode(ArchiveBlockDecodeError),
    PixelDecode(PixelDecodeError),
    ImageAssembly(ImageAssemblyError),
    PngEncode(PngEncodeError),
}

impl fmt::Display for ExtractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPrefix(error) => error.fmt(formatter),
            Self::Io {
                operation,
                path,
                source,
            } => write!(
                formatter,
                "failed to {operation} at {}: {source}",
                path.display()
            ),
            Self::IndexParse(error) => error.fmt(formatter),
            Self::BlockScan {
                file_number,
                source,
            } => write!(
                formatter,
                "failed to scan data file {file_number}: {source}"
            ),
            Self::InvalidLayout { diagnostics } => {
                write!(
                    formatter,
                    "archive layout could not be validated: {diagnostics:?}"
                )
            }
            Self::ResourceNotFound { key } => write!(
                formatter,
                "resource was not found: group={}, icon={}, block={}",
                key.group_code, key.icon_id, key.block_index
            ),
            Self::ConflictingRecordDimensions {
                key,
                first,
                conflicting,
            } => write!(
                formatter,
                "resource records disagree on dimensions for group={}, icon={}, block={}: {}x{} versus {}x{}",
                key.group_code,
                key.icon_id,
                key.block_index,
                first.0,
                first.1,
                conflicting.0,
                conflicting.1
            ),
            Self::AssemblyTileRecordNotFound { block_index } => write!(
                formatter,
                "assembly tile has no index record: block={block_index}"
            ),
            Self::ConflictingBlockDimensions {
                block_index,
                first,
                conflicting,
            } => write!(
                formatter,
                "assembly tile records disagree on dimensions for block={block_index}: {}x{} versus {}x{}",
                first.0, first.1, conflicting.0, conflicting.1
            ),
            Self::AssemblyArchiveMismatch { expected, actual } => write!(
                formatter,
                "assembly rule belongs to archive {expected}, not {actual}"
            ),
            Self::AssemblyRuleNotVerified {
                first_block,
                last_block,
            } => write!(
                formatter,
                "assembly rule for blocks {first_block}..={last_block} is not human-verified"
            ),
            Self::BlockUnavailable { block_index } => {
                write!(formatter, "resolved block {block_index} is unavailable")
            }
            Self::DataFileUnavailable { file_number } => {
                write!(formatter, "data file {file_number} is unavailable")
            }
            Self::BlockDecode(error) => error.fmt(formatter),
            Self::PixelDecode(error) => error.fmt(formatter),
            Self::ImageAssembly(error) => error.fmt(formatter),
            Self::PngEncode(error) => error.fmt(formatter),
        }
    }
}

impl Error for ExtractError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidPrefix(error) => Some(error),
            Self::Io { source, .. } => Some(source),
            Self::IndexParse(error) => Some(error),
            Self::BlockScan { source, .. } => Some(source),
            Self::BlockDecode(error) => Some(error),
            Self::PixelDecode(error) => Some(error),
            Self::ImageAssembly(error) => Some(error),
            Self::PngEncode(error) => Some(error),
            Self::InvalidLayout { .. }
            | Self::ResourceNotFound { .. }
            | Self::ConflictingRecordDimensions { .. }
            | Self::AssemblyTileRecordNotFound { .. }
            | Self::ConflictingBlockDimensions { .. }
            | Self::AssemblyArchiveMismatch { .. }
            | Self::AssemblyRuleNotVerified { .. }
            | Self::BlockUnavailable { .. }
            | Self::DataFileUnavailable { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let number = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "dho-vault-extract-test-{}-{number}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("create test directory");
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn push_u32(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_archive(directory: &Path, records: &[[u32; 5]], data: &[u8]) {
        let mut index = Vec::new();
        let group_count = records
            .iter()
            .map(|record| record[4])
            .collect::<std::collections::HashSet<_>>()
            .len() as u32;
        let block_count = records
            .iter()
            .map(|record| record[1])
            .max()
            .map_or(0, |block_index| block_index + 1);
        for value in [records.len() as u32, group_count, 1, 1, block_count, 1, 0] {
            push_u32(&mut index, value);
        }
        for record in records {
            for value in record {
                push_u32(&mut index, *value);
            }
        }
        fs::write(directory.join("sc000000.bin"), index).expect("write test index");
        fs::write(directory.join("sc000001.bin"), data).expect("write test data");
    }

    fn zlib_block(raw: &[u8]) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(raw).expect("write zlib input");
        let compressed = encoder.finish().expect("finish zlib stream");
        let mut block = b"MWC\x1a".to_vec();
        push_u32(&mut block, raw.len() as u32);
        push_u32(&mut block, compressed.len() as u32);
        block.extend_from_slice(&compressed);
        block
    }

    fn key() -> ResourceKey {
        ResourceKey {
            group_code: 9,
            icon_id: 7,
            block_index: 0,
        }
    }

    #[test]
    fn extracts_one_zlib_resource_as_png() {
        let directory = TestDirectory::new();
        write_archive(&directory.0, &[[7, 0, 1, 1, 9]], &zlib_block(&[1, 2, 3, 4]));
        let archive = LoadedArchive::open(&directory.0, "SC").expect("open test archive");

        let extracted = archive.extract_png(key(), 4).expect("extract PNG");

        assert_eq!(archive.prefix().as_str(), "sc");
        assert_eq!(archive.resource_keys().collect::<Vec<_>>(), [key()]);
        assert_eq!((extracted.width, extracted.height), (1, 1));
        assert_eq!(&extracted.png[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn extracts_one_raw_resource_through_the_same_api() {
        let directory = TestDirectory::new();
        write_archive(&directory.0, &[[7, 0, 1, 1, 9]], &[1, 2, 3, 4]);
        let archive = LoadedArchive::open(&directory.0, "sc").expect("open test archive");

        let extracted = archive.extract_png(key(), 4).expect("extract raw PNG");

        assert_eq!(&extracted.png[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn reads_the_data_file_only_when_a_resource_is_extracted() {
        let directory = TestDirectory::new();
        write_archive(&directory.0, &[[7, 0, 1, 1, 9]], &[1, 2, 3, 4]);
        let archive = LoadedArchive::open(&directory.0, "sc").expect("open test archive");
        fs::remove_file(directory.0.join("sc000001.bin")).expect("remove test data file");

        let error = archive.extract_png(key(), 4).unwrap_err();

        assert!(matches!(
            error,
            ExtractError::Io {
                operation: "read archive block",
                ..
            }
        ));
    }

    #[test]
    fn rejects_a_path_like_prefix() {
        let directory = TestDirectory::new();

        let error = LoadedArchive::open(&directory.0, "../").unwrap_err();

        assert!(matches!(error, ExtractError::InvalidPrefix(_)));
    }

    #[test]
    fn reports_a_missing_physical_resource_key() {
        let directory = TestDirectory::new();
        write_archive(&directory.0, &[[7, 0, 1, 1, 9]], &[1, 2, 3, 4]);
        let archive = LoadedArchive::open(&directory.0, "sc").expect("open test archive");
        let missing = ResourceKey {
            icon_id: 8,
            ..key()
        };

        let error = archive.extract_png(missing, 4).unwrap_err();

        assert!(matches!(error, ExtractError::ResourceNotFound { key } if key == missing));
    }

    #[test]
    fn rejects_conflicting_dimensions_for_the_same_physical_key() {
        let directory = TestDirectory::new();
        write_archive(
            &directory.0,
            &[[7, 0, 1, 2, 9], [7, 0, 2, 1, 9]],
            &[1, 2, 3, 4, 5, 6, 7, 8],
        );
        let archive = LoadedArchive::open(&directory.0, "sc").expect("open test archive");

        let error = archive.extract_png(key(), 8).unwrap_err();

        assert!(matches!(
            error,
            ExtractError::ConflictingRecordDimensions { .. }
        ));
    }

    #[test]
    fn decodes_and_joins_physical_blocks_from_an_assembly_plan() {
        let directory = TestDirectory::new();
        let records = [
            [100, 0, 2, 1, 9],
            [101, 1, 1, 1, 9],
            [102, 2, 2, 2, 9],
            [103, 3, 1, 2, 9],
        ];
        let raw_tiles = [
            vec![0, 0, 255, 255, 0, 0, 255, 255],
            vec![0, 255, 0, 255],
            [255, 0, 0, 255].repeat(4),
            [0, 255, 255, 255].repeat(2),
        ];
        let data = raw_tiles
            .iter()
            .flat_map(|raw| zlib_block(raw))
            .collect::<Vec<_>>();
        write_archive(&directory.0, &records, &data);
        let archive = LoadedArchive::open(&directory.0, "sc").expect("open test archive");
        let rule = dho_catalog::AssemblyRule {
            archive: "sc",
            start_block: 0,
            end_block: 3,
            tiles_per_image: 4,
            columns: 2,
            rows: 2,
            output_width: 3,
            output_height: 3,
            tile_order: dho_catalog::TileOrder::RowMajor,
            status: VerificationStatus::HumanVerified,
        };
        let plan = AssemblyPlan {
            rule,
            image_index: 0,
            first_block: 0,
            last_block: 3,
            tile_index: 0,
            row: 0,
            column: 0,
        };

        let assembled = archive
            .extract_assembly(plan, 16, 36)
            .expect("extract assembled PNG");
        let decoded = image::load_from_memory_with_format(&assembled.png, image::ImageFormat::Png)
            .expect("decode assembled PNG")
            .into_rgba8();

        assert_eq!((assembled.first_block, assembled.last_block), (0, 3));
        assert_eq!((assembled.width, assembled.height), (3, 3));
        assert_eq!(
            decoded.as_raw(),
            &[
                255, 0, 0, 255, 255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 0, 0, 255, 255,
                255, 255, 0, 255, 0, 0, 255, 255, 0, 0, 255, 255, 255, 255, 0, 255,
            ]
        );
    }

    #[test]
    fn returns_none_when_a_block_has_no_verified_assembly_rule() {
        let directory = TestDirectory::new();
        write_archive(&directory.0, &[[7, 0, 1, 1, 9]], &[1, 2, 3, 4]);
        let archive = LoadedArchive::open(&directory.0, "sc").expect("open test archive");

        let assembled = archive
            .extract_verified_assembly(0, 4, 4)
            .expect("look up verified assembly");

        assert_eq!(assembled, None);
    }
}
