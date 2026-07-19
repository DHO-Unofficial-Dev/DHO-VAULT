// SPDX-License-Identifier: MPL-2.0

//! Read-only loading and on-demand extraction of indexed DHO image resources.

use dho_catalog::{
    AssemblyPlan, LayeredAssemblyRule, VerificationStatus, assembly_candidate_plan, assembly_plan,
};
use dho_core::{
    ArchiveBlockDecodeError, ArchiveDiagnostic, ArchiveLayout, BlockDecodeError, BlockScanError,
    IndexParseError, IndexRecord, IndexedArchive, InlineBlockTable, InlineBlockTableError,
    MwcBlock, ScannedDataFile, build_archive_layout, scan_data_file,
};
use dho_image::{ImageAssemblyError, PixelDecodeError, PngEncodeError, RgbaImage, ThumbnailError};
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

/// A physical image block from an archive that has no separate index file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RawResourceKey {
    pub block_index: u32,
    pub file_number: u32,
    pub file_block_index: u32,
}

/// Pixel interpretation declared for a reviewed raw image archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawPixelFormat {
    Gray8,
    Bgra8,
}

/// One human-reviewed decoded size and its corresponding image dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawImageVariant {
    pub decoded_size: u32,
    pub width: u32,
    pub height: u32,
}

/// Human-reviewed dimensions and pixel interpretation for raw image blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawImageSpec {
    pub pixel_format: RawPixelFormat,
    pub variants: &'static [RawImageVariant],
}

/// How non-block bytes in one raw archive file are interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawArchiveLayout {
    BlocksOnly,
    InlineBlockTable,
}

/// Stable metadata for one physical block in a raw image archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawImageRecord {
    pub key: RawResourceKey,
    pub width: u32,
    pub height: u32,
}

/// One requested resource encoded for display or download.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedResource {
    pub key: ResourceKey,
    pub width: u32,
    pub height: u32,
    pub png: Vec<u8>,
}

/// One physical resource resized for a bounded gallery preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedThumbnail {
    pub key: ResourceKey,
    pub source_width: u32,
    pub source_height: u32,
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

/// One completed, human-verified image resized for a bounded gallery preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedAssemblyThumbnail {
    pub first_block: u32,
    pub last_block: u32,
    pub source_width: u32,
    pub source_height: u32,
    pub width: u32,
    pub height: u32,
    pub png: Vec<u8>,
}

#[derive(Debug)]
struct ArchiveDataFile {
    file_number: u32,
    path: PathBuf,
}

#[derive(Debug)]
struct RawImageBlock {
    record: RawImageRecord,
    block: MwcBlock,
    path: PathBuf,
}

/// An archive without a separate index, validated against a reviewed image specification.
#[derive(Debug)]
pub struct LoadedRawImageArchive {
    prefix: ArchivePrefix,
    spec: RawImageSpec,
    file_count: u32,
    blocks: Vec<RawImageBlock>,
}

impl LoadedRawImageArchive {
    /// Opens consecutive block-only files numbered from one.
    pub fn open(
        directory: impl AsRef<Path>,
        prefix: &str,
        archive_count: u32,
        spec: RawImageSpec,
    ) -> Result<Self, ExtractError> {
        let file_numbers = (1..=archive_count).collect::<Vec<_>>();
        Self::open_files(
            directory,
            prefix,
            &file_numbers,
            RawArchiveLayout::BlocksOnly,
            spec,
        )
    }

    /// Opens explicitly numbered files using one reviewed physical layout and image specification.
    pub fn open_files(
        directory: impl AsRef<Path>,
        prefix: &str,
        file_numbers: &[u32],
        layout: RawArchiveLayout,
        spec: RawImageSpec,
    ) -> Result<Self, ExtractError> {
        let directory = directory.as_ref();
        let prefix = ArchivePrefix::parse(prefix).map_err(ExtractError::InvalidPrefix)?;
        validate_raw_image_spec(spec)?;
        let mut blocks = Vec::new();
        let mut block_index = 0_u32;

        for &file_number in file_numbers {
            let path = directory.join(format!("{}{file_number:06}.bin", prefix.as_str()));
            let bytes = read_file("read raw image archive", &path)?;
            let scanned =
                scan_data_file(file_number, &bytes).map_err(|source| ExtractError::BlockScan {
                    file_number,
                    source,
                })?;
            validate_raw_archive_layout(layout, &bytes, &scanned)?;

            for (file_block_index, block) in scanned.zlib_blocks().copied().enumerate() {
                let variant = spec
                    .variants
                    .iter()
                    .find(|variant| variant.decoded_size == block.uncompressed_size)
                    .ok_or_else(|| ExtractError::RawImageSizeUnsupported {
                        file_number,
                        file_block_index: u32::try_from(file_block_index).unwrap_or(u32::MAX),
                        actual_size: block.uncompressed_size,
                        supported_sizes: spec
                            .variants
                            .iter()
                            .map(|variant| variant.decoded_size)
                            .collect(),
                    })?;
                let file_block_index = u32::try_from(file_block_index)
                    .map_err(|_| ExtractError::RawBlockIndexOverflow { file_number })?;
                let record = RawImageRecord {
                    key: RawResourceKey {
                        block_index,
                        file_number,
                        file_block_index,
                    },
                    width: variant.width,
                    height: variant.height,
                };
                blocks.push(RawImageBlock {
                    record,
                    block,
                    path: path.clone(),
                });
                block_index = block_index
                    .checked_add(1)
                    .ok_or(ExtractError::RawArchiveBlockCountOverflow)?;
            }
        }

        Ok(Self {
            prefix,
            spec,
            file_count: u32::try_from(file_numbers.len())
                .map_err(|_| ExtractError::RawArchiveFileCountOverflow)?,
            blocks,
        })
    }

    pub fn prefix(&self) -> &ArchivePrefix {
        &self.prefix
    }

    pub fn archive_count(&self) -> u32 {
        self.file_count
    }

    pub fn records(&self) -> impl Iterator<Item = RawImageRecord> + '_ {
        self.blocks.iter().map(|block| block.record)
    }

    pub fn extract_png(
        &self,
        key: RawResourceKey,
        max_output_size: usize,
    ) -> Result<ExtractedResource, ExtractError> {
        let block = self.raw_block(key)?;
        let image = self.decode_raw_block(block, max_output_size)?;
        let png = image.encode_png().map_err(ExtractError::PngEncode)?;
        Ok(ExtractedResource {
            key: ResourceKey {
                group_code: 0,
                icon_id: key.block_index,
                block_index: key.block_index,
            },
            width: block.record.width,
            height: block.record.height,
            png,
        })
    }

    pub fn extract_thumbnail_png(
        &self,
        key: RawResourceKey,
        max_decode_size: usize,
        max_width: u32,
        max_height: u32,
        max_thumbnail_output_size: usize,
    ) -> Result<ExtractedThumbnail, ExtractError> {
        let block = self.raw_block(key)?;
        let image = self.decode_raw_block(block, max_decode_size)?;
        let thumbnail = image
            .thumbnail(max_width, max_height, max_thumbnail_output_size)
            .map_err(ExtractError::Thumbnail)?;
        let png = thumbnail.encode_png().map_err(ExtractError::PngEncode)?;
        Ok(ExtractedThumbnail {
            key: ResourceKey {
                group_code: 0,
                icon_id: key.block_index,
                block_index: key.block_index,
            },
            source_width: block.record.width,
            source_height: block.record.height,
            width: thumbnail.width(),
            height: thumbnail.height(),
            png,
        })
    }

    /// Extracts one human-verified logical image from matching base and overlay raw tiles.
    pub fn extract_verified_layered_assembly(
        &self,
        rule: LayeredAssemblyRule,
        max_tile_output_size: usize,
        max_assembled_output_size: usize,
    ) -> Result<ExtractedAssembly, ExtractError> {
        let image =
            self.decode_layered_assembly(rule, max_tile_output_size, max_assembled_output_size)?;
        let png = image.encode_png().map_err(ExtractError::PngEncode)?;
        Ok(ExtractedAssembly {
            first_block: rule.canonical_block,
            last_block: rule.last_block,
            width: image.width(),
            height: image.height(),
            png,
        })
    }

    /// Extracts a bounded preview of one human-verified layered raw-image assembly.
    pub fn extract_verified_layered_assembly_thumbnail(
        &self,
        rule: LayeredAssemblyRule,
        max_tile_output_size: usize,
        max_assembled_output_size: usize,
        max_width: u32,
        max_height: u32,
        max_thumbnail_output_size: usize,
    ) -> Result<ExtractedAssemblyThumbnail, ExtractError> {
        let image =
            self.decode_layered_assembly(rule, max_tile_output_size, max_assembled_output_size)?;
        let source_width = image.width();
        let source_height = image.height();
        let thumbnail = image
            .thumbnail(max_width, max_height, max_thumbnail_output_size)
            .map_err(ExtractError::Thumbnail)?;
        let png = thumbnail.encode_png().map_err(ExtractError::PngEncode)?;
        Ok(ExtractedAssemblyThumbnail {
            first_block: rule.canonical_block,
            last_block: rule.last_block,
            source_width,
            source_height,
            width: thumbnail.width(),
            height: thumbnail.height(),
            png,
        })
    }

    fn decode_layered_assembly(
        &self,
        rule: LayeredAssemblyRule,
        max_tile_output_size: usize,
        max_assembled_output_size: usize,
    ) -> Result<RgbaImage, ExtractError> {
        if !rule.archive.eq_ignore_ascii_case(self.prefix.as_str()) {
            return Err(ExtractError::AssemblyArchiveMismatch {
                expected: rule.archive,
                actual: self.prefix.as_str().to_owned(),
            });
        }
        if rule.status != VerificationStatus::HumanVerified {
            return Err(ExtractError::AssemblyRuleNotVerified {
                first_block: rule.canonical_block,
                last_block: rule.last_block,
            });
        }

        let mut tiles = Vec::with_capacity(
            usize::try_from(rule.tile_count)
                .map_err(|_| ExtractError::RawArchiveBlockCountOverflow)?,
        );
        let first_index = rule.first_file_block_index;
        let base_path = self
            .raw_block_in_file(rule.base_file_number, first_index)?
            .path
            .clone();
        let overlay_path = self
            .raw_block_in_file(rule.overlay_file_number, first_index)?
            .path
            .clone();
        let base_bytes = read_file("read raw assembly base layer", &base_path)?;
        let overlay_bytes = read_file("read raw assembly overlay layer", &overlay_path)?;
        for offset in 0..rule.tile_count {
            let file_block_index = rule
                .first_file_block_index
                .checked_add(offset)
                .ok_or(ExtractError::RawArchiveBlockCountOverflow)?;
            let base = self.raw_block_in_file(rule.base_file_number, file_block_index)?;
            let layer = self.raw_block_in_file(rule.overlay_file_number, file_block_index)?;
            let mut combined =
                self.decode_raw_block_bytes(base, &base_bytes, max_tile_output_size)?;
            let layer = self.decode_raw_block_bytes(layer, &overlay_bytes, max_tile_output_size)?;
            combined
                .overlay(&layer)
                .map_err(ExtractError::ImageAssembly)?;
            tiles.push(combined);
        }

        RgbaImage::assemble_grid(
            &tiles,
            rule.columns,
            rule.rows,
            rule.output_width,
            rule.output_height,
            max_assembled_output_size,
        )
        .map_err(ExtractError::ImageAssembly)
    }

    fn raw_block(&self, key: RawResourceKey) -> Result<&RawImageBlock, ExtractError> {
        self.blocks
            .get(usize::try_from(key.block_index).unwrap_or(usize::MAX))
            .filter(|block| block.record.key == key)
            .ok_or(ExtractError::RawResourceNotFound { key })
    }

    fn raw_block_in_file(
        &self,
        file_number: u32,
        file_block_index: u32,
    ) -> Result<&RawImageBlock, ExtractError> {
        self.blocks
            .iter()
            .find(|block| {
                block.record.key.file_number == file_number
                    && block.record.key.file_block_index == file_block_index
            })
            .ok_or(ExtractError::RawLayerBlockMissing {
                file_number,
                file_block_index,
            })
    }

    fn decode_raw_block(
        &self,
        block: &RawImageBlock,
        max_output_size: usize,
    ) -> Result<RgbaImage, ExtractError> {
        let payload = read_file_range(
            "read raw image block",
            &block.path,
            block.block.payload_offset,
            usize::try_from(block.block.compressed_size).map_err(|_| {
                ExtractError::RawCompressedSizeOverflow {
                    key: block.record.key,
                    compressed_size: block.block.compressed_size,
                }
            })?,
        )?;
        let decoded = block
            .block
            .decode_payload(&payload, max_output_size)
            .map_err(ExtractError::RawBlockDecode)?;
        match self.spec.pixel_format {
            RawPixelFormat::Gray8 => {
                RgbaImage::from_gray8(block.record.width, block.record.height, &decoded)
                    .map_err(ExtractError::PixelDecode)
            }
            RawPixelFormat::Bgra8 => {
                RgbaImage::from_bgra(block.record.width, block.record.height, &decoded)
                    .map_err(ExtractError::PixelDecode)
            }
        }
    }

    fn decode_raw_block_bytes(
        &self,
        block: &RawImageBlock,
        bytes: &[u8],
        max_output_size: usize,
    ) -> Result<RgbaImage, ExtractError> {
        let decoded = block
            .block
            .decode(bytes, max_output_size)
            .map_err(ExtractError::RawBlockDecode)?;
        match self.spec.pixel_format {
            RawPixelFormat::Gray8 => {
                RgbaImage::from_gray8(block.record.width, block.record.height, &decoded)
                    .map_err(ExtractError::PixelDecode)
            }
            RawPixelFormat::Bgra8 => {
                RgbaImage::from_bgra(block.record.width, block.record.height, &decoded)
                    .map_err(ExtractError::PixelDecode)
            }
        }
    }
}

fn validate_raw_image_spec(spec: RawImageSpec) -> Result<(), ExtractError> {
    if spec.variants.is_empty() {
        return Err(ExtractError::RawImageSpecEmpty);
    }
    for variant in spec.variants {
        let expected_size = raw_image_byte_len(*variant, spec.pixel_format)?;
        if usize::try_from(variant.decoded_size).ok() != Some(expected_size) {
            return Err(ExtractError::RawImageSpecSizeMismatch {
                width: variant.width,
                height: variant.height,
                pixel_format: spec.pixel_format,
                declared_size: variant.decoded_size,
                expected_size,
            });
        }
    }
    Ok(())
}

fn raw_image_byte_len(
    variant: RawImageVariant,
    pixel_format: RawPixelFormat,
) -> Result<usize, ExtractError> {
    let pixels = usize::try_from(variant.width)
        .ok()
        .and_then(|width| {
            usize::try_from(variant.height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .ok_or(ExtractError::RawImageDimensionOverflow {
            width: variant.width,
            height: variant.height,
        })?;
    match pixel_format {
        RawPixelFormat::Gray8 => Ok(pixels),
        RawPixelFormat::Bgra8 => {
            pixels
                .checked_mul(4)
                .ok_or(ExtractError::RawImageDimensionOverflow {
                    width: variant.width,
                    height: variant.height,
                })
        }
    }
}

fn validate_raw_archive_layout(
    layout: RawArchiveLayout,
    bytes: &[u8],
    scanned: &ScannedDataFile,
) -> Result<(), ExtractError> {
    match layout {
        RawArchiveLayout::BlocksOnly => {
            if let Some(gap) = scanned.unresolved_gaps().next() {
                return Err(ExtractError::RawUnresolvedGap {
                    file_number: scanned.file_number,
                    offset: gap.location.offset,
                    len: gap.len,
                });
            }
        }
        RawArchiveLayout::InlineBlockTable => {
            let table = InlineBlockTable::parse(bytes).map_err(|source| {
                ExtractError::InlineBlockTableParse {
                    file_number: scanned.file_number,
                    source,
                }
            })?;
            let gaps = scanned.unresolved_gaps().collect::<Vec<_>>();
            if gaps.len() != 1 || gaps[0].location.offset != 0 || gaps[0].len != table.byte_len {
                return Err(ExtractError::InlineBlockTableGapMismatch {
                    file_number: scanned.file_number,
                    table_size: table.byte_len,
                    gaps: gaps
                        .into_iter()
                        .map(|gap| (gap.location.offset, gap.len))
                        .collect(),
                });
            }
            let blocks = scanned.zlib_blocks().collect::<Vec<_>>();
            if table.entries.len() != blocks.len() {
                return Err(ExtractError::InlineBlockCountMismatch {
                    file_number: scanned.file_number,
                    table_count: table.entries.len(),
                    block_count: blocks.len(),
                });
            }
            for (index, (entry, block)) in table.entries.iter().zip(blocks).enumerate() {
                let actual_stored_size = usize::try_from(block.compressed_size)
                    .ok()
                    .and_then(|size| size.checked_add(12))
                    .ok_or(ExtractError::RawCompressedSizeOverflow {
                        key: RawResourceKey {
                            block_index: u32::try_from(index).unwrap_or(u32::MAX),
                            file_number: scanned.file_number,
                            file_block_index: u32::try_from(index).unwrap_or(u32::MAX),
                        },
                        compressed_size: block.compressed_size,
                    })?;
                if usize::try_from(entry.offset).ok() != Some(block.location.offset)
                    || usize::try_from(entry.stored_size).ok() != Some(actual_stored_size)
                {
                    return Err(ExtractError::InlineBlockEntryMismatch {
                        file_number: scanned.file_number,
                        file_block_index: u32::try_from(index).unwrap_or(u32::MAX),
                        table_offset: entry.offset,
                        table_stored_size: entry.stored_size,
                        block_offset: block.location.offset,
                        block_stored_size: actual_stored_size,
                    });
                }
            }
        }
    }
    Ok(())
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
        let record = self.record_for_key(key)?;
        let image = self.decode_record(record, max_output_size)?;
        let png = image.encode_png().map_err(ExtractError::PngEncode)?;

        Ok(ExtractedResource {
            key,
            width: record.width,
            height: record.height,
            png,
        })
    }

    /// Decodes one physical record and returns a bounded PNG thumbnail.
    pub fn extract_thumbnail_png(
        &self,
        key: ResourceKey,
        max_decode_size: usize,
        max_width: u32,
        max_height: u32,
        max_thumbnail_output_size: usize,
    ) -> Result<ExtractedThumbnail, ExtractError> {
        let record = self.record_for_key(key)?;
        let image = self.decode_record(record, max_decode_size)?;
        let thumbnail = image
            .thumbnail(max_width, max_height, max_thumbnail_output_size)
            .map_err(ExtractError::Thumbnail)?;
        let png = thumbnail.encode_png().map_err(ExtractError::PngEncode)?;

        Ok(ExtractedThumbnail {
            key,
            source_width: record.width,
            source_height: record.height,
            width: thumbnail.width(),
            height: thumbnail.height(),
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

    /// Joins an unverified candidate for Curator review without exposing it to Viewer flows.
    pub fn extract_candidate_assembly(
        &self,
        block_index: u32,
        max_tile_output_size: usize,
        max_assembled_output_size: usize,
    ) -> Result<Option<ExtractedAssembly>, ExtractError> {
        let Some(plan) = assembly_candidate_plan(self.prefix.as_str(), block_index) else {
            return Ok(None);
        };

        self.extract_assembly_with_status(
            plan,
            VerificationStatus::Candidate,
            max_tile_output_size,
            max_assembled_output_size,
        )
        .map(Some)
    }

    /// Joins a verified image and returns only a bounded PNG thumbnail.
    pub fn extract_verified_assembly_thumbnail(
        &self,
        block_index: u32,
        max_tile_output_size: usize,
        max_assembled_output_size: usize,
        max_width: u32,
        max_height: u32,
        max_thumbnail_output_size: usize,
    ) -> Result<Option<ExtractedAssemblyThumbnail>, ExtractError> {
        let Some(plan) = assembly_plan(self.prefix.as_str(), block_index) else {
            return Ok(None);
        };
        self.extract_assembly_thumbnail(
            plan,
            max_tile_output_size,
            max_assembled_output_size,
            max_width,
            max_height,
            max_thumbnail_output_size,
        )
        .map(Some)
    }

    fn extract_assembly_thumbnail(
        &self,
        plan: AssemblyPlan,
        max_tile_output_size: usize,
        max_assembled_output_size: usize,
        max_width: u32,
        max_height: u32,
        max_thumbnail_output_size: usize,
    ) -> Result<ExtractedAssemblyThumbnail, ExtractError> {
        let image = self.decode_assembly(
            plan,
            VerificationStatus::HumanVerified,
            max_tile_output_size,
            max_assembled_output_size,
        )?;
        let thumbnail = image
            .thumbnail(max_width, max_height, max_thumbnail_output_size)
            .map_err(ExtractError::Thumbnail)?;
        let png = thumbnail.encode_png().map_err(ExtractError::PngEncode)?;

        Ok(ExtractedAssemblyThumbnail {
            first_block: plan.first_block,
            last_block: plan.last_block,
            source_width: image.width(),
            source_height: image.height(),
            width: thumbnail.width(),
            height: thumbnail.height(),
            png,
        })
    }

    fn extract_assembly(
        &self,
        plan: AssemblyPlan,
        max_tile_output_size: usize,
        max_assembled_output_size: usize,
    ) -> Result<ExtractedAssembly, ExtractError> {
        self.extract_assembly_with_status(
            plan,
            VerificationStatus::HumanVerified,
            max_tile_output_size,
            max_assembled_output_size,
        )
    }

    fn extract_assembly_with_status(
        &self,
        plan: AssemblyPlan,
        expected_status: VerificationStatus,
        max_tile_output_size: usize,
        max_assembled_output_size: usize,
    ) -> Result<ExtractedAssembly, ExtractError> {
        let image = self.decode_assembly(
            plan,
            expected_status,
            max_tile_output_size,
            max_assembled_output_size,
        )?;
        let png = image.encode_png().map_err(ExtractError::PngEncode)?;

        Ok(ExtractedAssembly {
            first_block: plan.first_block,
            last_block: plan.last_block,
            width: image.width(),
            height: image.height(),
            png,
        })
    }

    fn decode_assembly(
        &self,
        plan: AssemblyPlan,
        expected_status: VerificationStatus,
        max_tile_output_size: usize,
        max_assembled_output_size: usize,
    ) -> Result<RgbaImage, ExtractError> {
        if !plan.rule.archive.eq_ignore_ascii_case(self.prefix.as_str()) {
            return Err(ExtractError::AssemblyArchiveMismatch {
                expected: plan.rule.archive,
                actual: self.prefix.as_str().to_owned(),
            });
        }
        if plan.rule.status != expected_status {
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

        RgbaImage::assemble_grid(
            &tiles,
            plan.rule.columns,
            plan.rule.rows,
            plan.rule.output_width,
            plan.rule.output_height,
            max_assembled_output_size,
        )
        .map_err(ExtractError::ImageAssembly)
    }

    fn record_for_key(&self, key: ResourceKey) -> Result<IndexRecord, ExtractError> {
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
        Ok(record)
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
    RawImageDimensionOverflow {
        width: u32,
        height: u32,
    },
    RawImageSpecEmpty,
    RawImageSpecSizeMismatch {
        width: u32,
        height: u32,
        pixel_format: RawPixelFormat,
        declared_size: u32,
        expected_size: usize,
    },
    RawUnresolvedGap {
        file_number: u32,
        offset: usize,
        len: usize,
    },
    RawImageSizeUnsupported {
        file_number: u32,
        file_block_index: u32,
        actual_size: u32,
        supported_sizes: Vec<u32>,
    },
    RawBlockIndexOverflow {
        file_number: u32,
    },
    RawArchiveBlockCountOverflow,
    RawArchiveFileCountOverflow,
    InlineBlockTableParse {
        file_number: u32,
        source: InlineBlockTableError,
    },
    InlineBlockTableGapMismatch {
        file_number: u32,
        table_size: usize,
        gaps: Vec<(usize, usize)>,
    },
    InlineBlockCountMismatch {
        file_number: u32,
        table_count: usize,
        block_count: usize,
    },
    InlineBlockEntryMismatch {
        file_number: u32,
        file_block_index: u32,
        table_offset: u32,
        table_stored_size: u32,
        block_offset: usize,
        block_stored_size: usize,
    },
    RawResourceNotFound {
        key: RawResourceKey,
    },
    RawLayerBlockMissing {
        file_number: u32,
        file_block_index: u32,
    },
    RawCompressedSizeOverflow {
        key: RawResourceKey,
        compressed_size: u32,
    },
    RawBlockDecode(BlockDecodeError),
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
    Thumbnail(ThumbnailError),
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
            Self::RawImageDimensionOverflow { width, height } => write!(
                formatter,
                "raw image dimensions overflow this platform: {width}x{height}"
            ),
            Self::RawImageSpecEmpty => write!(formatter, "raw image specification has no sizes"),
            Self::RawImageSpecSizeMismatch {
                width,
                height,
                pixel_format,
                declared_size,
                expected_size,
            } => write!(
                formatter,
                "raw image specification disagrees with its pixel format: {width}x{height} {pixel_format:?} declares {declared_size} bytes, expected {expected_size}"
            ),
            Self::RawUnresolvedGap {
                file_number,
                offset,
                len,
            } => write!(
                formatter,
                "raw image archive file {file_number} contains {len} unresolved bytes at offset {offset}"
            ),
            Self::RawImageSizeUnsupported {
                file_number,
                file_block_index,
                actual_size,
                supported_sizes,
            } => write!(
                formatter,
                "raw image block {file_block_index} in file {file_number} declares unsupported size {actual_size}; supported sizes are {supported_sizes:?}"
            ),
            Self::RawBlockIndexOverflow { file_number } => write!(
                formatter,
                "raw image block index exceeds u32 in file {file_number}"
            ),
            Self::RawArchiveBlockCountOverflow => {
                write!(formatter, "raw image archive block count exceeds u32")
            }
            Self::RawArchiveFileCountOverflow => {
                write!(formatter, "raw image archive file count exceeds u32")
            }
            Self::InlineBlockTableParse {
                file_number,
                source,
            } => write!(
                formatter,
                "failed to parse inline block table in file {file_number}: {source}"
            ),
            Self::InlineBlockTableGapMismatch {
                file_number,
                table_size,
                gaps,
            } => write!(
                formatter,
                "inline block table in file {file_number} occupies {table_size} bytes but unresolved ranges are {gaps:?}"
            ),
            Self::InlineBlockCountMismatch {
                file_number,
                table_count,
                block_count,
            } => write!(
                formatter,
                "inline block table in file {file_number} declares {table_count} blocks but the file contains {block_count}"
            ),
            Self::InlineBlockEntryMismatch {
                file_number,
                file_block_index,
                table_offset,
                table_stored_size,
                block_offset,
                block_stored_size,
            } => write!(
                formatter,
                "inline block entry {file_block_index} in file {file_number} does not match its MWC block: table=({table_offset}, {table_stored_size}), block=({block_offset}, {block_stored_size})"
            ),
            Self::RawResourceNotFound { key } => write!(
                formatter,
                "raw image block was not found: archive block {}, file {}, file block {}",
                key.block_index, key.file_number, key.file_block_index
            ),
            Self::RawLayerBlockMissing {
                file_number,
                file_block_index,
            } => write!(
                formatter,
                "raw assembly layer block was not found: file {file_number}, file block {file_block_index}"
            ),
            Self::RawCompressedSizeOverflow {
                key,
                compressed_size,
            } => write!(
                formatter,
                "raw image block {} compressed size {compressed_size} exceeds this platform",
                key.block_index
            ),
            Self::RawBlockDecode(error) => error.fmt(formatter),
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
            Self::Thumbnail(error) => error.fmt(formatter),
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
            Self::InlineBlockTableParse { source, .. } => Some(source),
            Self::RawBlockDecode(error) => Some(error),
            Self::BlockDecode(error) => Some(error),
            Self::PixelDecode(error) => Some(error),
            Self::ImageAssembly(error) => Some(error),
            Self::Thumbnail(error) => Some(error),
            Self::PngEncode(error) => Some(error),
            Self::RawImageDimensionOverflow { .. }
            | Self::RawImageSpecEmpty
            | Self::RawImageSpecSizeMismatch { .. }
            | Self::RawUnresolvedGap { .. }
            | Self::RawImageSizeUnsupported { .. }
            | Self::RawBlockIndexOverflow { .. }
            | Self::RawArchiveBlockCountOverflow
            | Self::RawArchiveFileCountOverflow
            | Self::InlineBlockTableGapMismatch { .. }
            | Self::InlineBlockCountMismatch { .. }
            | Self::InlineBlockEntryMismatch { .. }
            | Self::RawResourceNotFound { .. }
            | Self::RawLayerBlockMissing { .. }
            | Self::RawCompressedSizeOverflow { .. }
            | Self::InvalidLayout { .. }
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

    const GRAY8_2X1_VARIANTS: &[RawImageVariant] = &[RawImageVariant {
        decoded_size: 2,
        width: 2,
        height: 1,
    }];
    const GRAY8_2X2_VARIANTS: &[RawImageVariant] = &[RawImageVariant {
        decoded_size: 4,
        width: 2,
        height: 2,
    }];
    const BGRA8_VARIANTS: &[RawImageVariant] = &[
        RawImageVariant {
            decoded_size: 8,
            width: 2,
            height: 1,
        },
        RawImageVariant {
            decoded_size: 16,
            width: 2,
            height: 2,
        },
    ];
    const BGRA8_1X1_VARIANTS: &[RawImageVariant] = &[RawImageVariant {
        decoded_size: 4,
        width: 1,
        height: 1,
    }];

    fn gray8_spec(variants: &'static [RawImageVariant]) -> RawImageSpec {
        RawImageSpec {
            pixel_format: RawPixelFormat::Gray8,
            variants,
        }
    }

    fn inline_archive(raw_blocks: &[Vec<u8>]) -> Vec<u8> {
        let blocks = raw_blocks
            .iter()
            .map(|raw| zlib_block(raw))
            .collect::<Vec<_>>();
        let mut offset = 4 + blocks.len() * 8;
        let mut bytes = Vec::new();
        push_u32(&mut bytes, blocks.len() as u32);
        for block in &blocks {
            push_u32(&mut bytes, offset as u32);
            push_u32(&mut bytes, block.len() as u32);
            offset += block.len();
        }
        for block in blocks {
            bytes.extend_from_slice(&block);
        }
        bytes
    }

    #[test]
    fn opens_and_extracts_indexless_grayscale_blocks() {
        let directory = TestDirectory::new();
        let mut data = zlib_block(&[0x11, 0x22]);
        data.extend_from_slice(&zlib_block(&[0x33, 0x44]));
        fs::write(directory.0.join("sh000001.bin"), data).expect("write raw archive");

        let archive =
            LoadedRawImageArchive::open(&directory.0, "SH", 1, gray8_spec(GRAY8_2X1_VARIANTS))
                .expect("open raw archive");
        let records = archive.records().collect::<Vec<_>>();

        assert_eq!(archive.prefix().as_str(), "sh");
        assert_eq!(archive.archive_count(), 1);
        assert_eq!(records.len(), 2);
        assert_eq!(
            records[1].key,
            RawResourceKey {
                block_index: 1,
                file_number: 1,
                file_block_index: 1,
            }
        );
        let extracted = archive
            .extract_png(records[0].key, 2)
            .expect("extract raw PNG");
        assert_eq!((extracted.width, extracted.height), (2, 1));
        assert_eq!(&extracted.png[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn rejects_raw_blocks_that_do_not_match_the_reviewed_dimensions() {
        let directory = TestDirectory::new();
        fs::write(directory.0.join("sh000001.bin"), zlib_block(&[0; 3]))
            .expect("write raw archive");

        let error =
            LoadedRawImageArchive::open(&directory.0, "sh", 1, gray8_spec(GRAY8_2X2_VARIANTS))
                .unwrap_err();

        assert!(matches!(
            error,
            ExtractError::RawImageSizeUnsupported {
                file_number: 1,
                file_block_index: 0,
                actual_size: 3,
                ref supported_sizes,
            } if supported_sizes == &[4]
        ));
    }

    #[test]
    fn rejects_unresolved_bytes_in_a_raw_archive() {
        let directory = TestDirectory::new();
        let mut data = vec![0xff];
        data.extend_from_slice(&zlib_block(&[0; 4]));
        fs::write(directory.0.join("sh000001.bin"), data).expect("write raw archive");

        let error =
            LoadedRawImageArchive::open(&directory.0, "sh", 1, gray8_spec(GRAY8_2X2_VARIANTS))
                .unwrap_err();

        assert!(matches!(
            error,
            ExtractError::RawUnresolvedGap {
                file_number: 1,
                offset: 0,
                len: 1,
            }
        ));
    }

    #[test]
    fn opens_inline_table_bgra_blocks_with_size_based_dimensions() {
        let directory = TestDirectory::new();
        fs::write(
            directory.0.join("tm000000.bin"),
            inline_archive(&[vec![1, 2, 3, 255, 4, 5, 6, 255], vec![7; 16]]),
        )
        .expect("write inline archive");
        let spec = RawImageSpec {
            pixel_format: RawPixelFormat::Bgra8,
            variants: BGRA8_VARIANTS,
        };

        let archive = LoadedRawImageArchive::open_files(
            &directory.0,
            "tm",
            &[0],
            RawArchiveLayout::InlineBlockTable,
            spec,
        )
        .expect("open inline BGRA archive");
        let records = archive.records().collect::<Vec<_>>();

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].key.file_number, 0);
        assert_eq!((records[0].width, records[0].height), (2, 1));
        assert_eq!((records[1].width, records[1].height), (2, 2));
        let extracted = archive
            .extract_png(records[0].key, 8)
            .expect("extract inline BGRA PNG");
        assert_eq!((extracted.width, extracted.height), (2, 1));
        assert_eq!(&extracted.png[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn rejects_an_inline_entry_that_disagrees_with_its_mwc_block() {
        let directory = TestDirectory::new();
        let mut data = inline_archive(&[vec![1, 2, 3, 255, 4, 5, 6, 255]]);
        let stored_size = u32::from_le_bytes(data[8..12].try_into().expect("stored size bytes"));
        data[8..12].copy_from_slice(&(stored_size - 1).to_le_bytes());
        fs::write(directory.0.join("tm000000.bin"), data).expect("write inline archive");
        let spec = RawImageSpec {
            pixel_format: RawPixelFormat::Bgra8,
            variants: BGRA8_VARIANTS,
        };

        let error = LoadedRawImageArchive::open_files(
            &directory.0,
            "tm",
            &[0],
            RawArchiveLayout::InlineBlockTable,
            spec,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            ExtractError::InlineBlockEntryMismatch {
                file_number: 0,
                file_block_index: 0,
                ..
            }
        ));
    }

    #[test]
    fn alpha_composites_and_assembles_matching_raw_file_layers() {
        let directory = TestDirectory::new();
        fs::write(
            directory.0.join("kp000000.bin"),
            inline_archive(&[vec![30, 20, 10, 255], vec![60, 50, 40, 255]]),
        )
        .expect("write base layer");
        fs::write(
            directory.0.join("kp000010.bin"),
            inline_archive(&[vec![0, 0, 0, 0], vec![90, 80, 70, 255]]),
        )
        .expect("write overlay layer");
        let archive = LoadedRawImageArchive::open_files(
            &directory.0,
            "kp",
            &[0, 10],
            RawArchiveLayout::InlineBlockTable,
            RawImageSpec {
                pixel_format: RawPixelFormat::Bgra8,
                variants: BGRA8_1X1_VARIANTS,
            },
        )
        .expect("open layered archive");
        let rule = LayeredAssemblyRule {
            archive: "kp",
            base_file_number: 0,
            overlay_file_number: 10,
            first_file_block_index: 0,
            tile_count: 2,
            columns: 2,
            rows: 1,
            output_width: 2,
            output_height: 1,
            canonical_block: 0,
            last_block: 3,
            status: VerificationStatus::HumanVerified,
        };

        let extracted = archive
            .extract_verified_layered_assembly(rule, 4, 8)
            .expect("extract layered assembly");
        let decoded = image::load_from_memory_with_format(&extracted.png, image::ImageFormat::Png)
            .expect("decode layered PNG")
            .to_rgba8();

        assert_eq!((extracted.width, extracted.height), (2, 1));
        assert_eq!(decoded.get_pixel(0, 0).0, [10, 20, 30, 255]);
        assert_eq!(decoded.get_pixel(1, 0).0, [70, 80, 90, 255]);
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
    fn extracts_one_resource_as_a_bounded_thumbnail() {
        let directory = TestDirectory::new();
        write_archive(
            &directory.0,
            &[[7, 0, 2, 1, 9]],
            &zlib_block(&[1, 2, 3, 255, 4, 5, 6, 255]),
        );
        let archive = LoadedArchive::open(&directory.0, "sc").expect("open test archive");

        let thumbnail = archive
            .extract_thumbnail_png(key(), 8, 1, 1, 4)
            .expect("extract thumbnail");

        assert_eq!((thumbnail.source_width, thumbnail.source_height), (2, 1));
        assert_eq!((thumbnail.width, thumbnail.height), (1, 1));
        assert_eq!(&thumbnail.png[..8], b"\x89PNG\r\n\x1a\n");
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

        let thumbnail = archive
            .extract_assembly_thumbnail(plan, 16, 36, 2, 2, 16)
            .expect("extract assembled thumbnail");
        assert_eq!((thumbnail.source_width, thumbnail.source_height), (3, 3));
        assert_eq!((thumbnail.width, thumbnail.height), (2, 2));
        assert_eq!(&thumbnail.png[..8], b"\x89PNG\r\n\x1a\n");
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
