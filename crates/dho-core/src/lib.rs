// SPDX-License-Identifier: MPL-2.0

//! Read-only parsers for DHO client resource archives.

pub mod index;

pub use index::{ArchiveHeader, IndexParseError, IndexRecord, IndexedArchive};
