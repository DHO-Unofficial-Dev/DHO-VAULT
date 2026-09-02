// SPDX-License-Identifier: MPL-2.0

//! Read-only parsers for DHO client resource archives.

pub mod archive;
pub mod audio;
pub mod block;
pub mod gm;
pub mod index;
pub mod inline;
pub mod license_terms;
pub mod master_text;
pub mod special_image;
pub mod text;

pub use archive::{
    ArchiveBlock, ArchiveBlockDecodeError, ArchiveBlockKind, ArchiveDiagnostic, ArchiveLayout,
    RawBlock, build_archive_layout,
};
pub use audio::{DecodedKovsAudio, KOVS_HEADER_SIZE, KovsHeader, KovsParseError, decode_kovs};
pub use block::{
    BlockDecodeError, BlockLocation, BlockScanError, DataSegment, MwcBlock, ScannedDataFile,
    UnresolvedGap, scan_data_file,
};
pub use gm::{GmAtlas, GmAtlasImage, GmAtlasParseError, GmAtlasSprite};
pub use index::{ArchiveHeader, IndexParseError, IndexRecord, IndexedArchive};
pub use inline::{InlineBlockEntry, InlineBlockTable, InlineBlockTableError};
pub use license_terms::{LicenseTermsArchive, LicenseTermsBlock, LicenseTermsParseError};
pub use master_text::{
    DtMasterData, DtMasterFieldKind, DtMasterParseError, DtMasterRecord, DtMasterTableSpec,
    DtMasterValue,
};
pub use special_image::{
    BitmapFontArchive, CursorArchive, CursorImage, FontGlyph, SpecialImageParseError, XftxArchive,
    XftxTexture, cursor_dimensions,
};
pub use text::{DtTextParseError, DtTextRecord, DtTextTable};
