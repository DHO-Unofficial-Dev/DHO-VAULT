// SPDX-License-Identifier: MPL-2.0

//! Read-only parsers for DHO client resource archives.

pub mod archive;
pub mod block;
pub mod index;

pub use archive::{
    ArchiveBlock, ArchiveBlockDecodeError, ArchiveBlockKind, ArchiveDiagnostic, ArchiveLayout,
    RawBlock, build_archive_layout,
};
pub use block::{
    BlockDecodeError, BlockLocation, BlockScanError, DataSegment, MwcBlock, ScannedDataFile,
    UnresolvedGap, scan_data_file,
};
pub use index::{ArchiveHeader, IndexParseError, IndexRecord, IndexedArchive};
