// SPDX-License-Identifier: MPL-2.0

//! Read-only parsing for the locale-specific compiled license tables in `lc000000.bin`.

use flate2::read::ZlibDecoder;
use std::error::Error;
use std::fmt;
use std::io::{self, Read};

const MWC_MAGIC: &[u8; 4] = b"MWC\x1a";
const MWC_HEADER_SIZE: usize = 12;
const TERMS_IDENTIFIER_SIZE: usize = 18;
const TERMS_HEADER_SIZE: usize = TERMS_IDENTIFIER_SIZE + 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicenseTermsBlock {
    pub index: usize,
    pub identifier: String,
    pub units: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicenseTermsArchive {
    pub blocks: Vec<LicenseTermsBlock>,
}

impl LicenseTermsArchive {
    pub fn parse(bytes: &[u8]) -> Result<Self, LicenseTermsParseError> {
        let block_count = usize::from(
            *bytes
                .first()
                .ok_or(LicenseTermsParseError::TruncatedHeader)?,
        );
        let offset_count = block_count
            .checked_add(1)
            .ok_or(LicenseTermsParseError::InvalidOffsets)?;
        let header_size = 1usize
            .checked_add(
                offset_count
                    .checked_mul(4)
                    .ok_or(LicenseTermsParseError::InvalidOffsets)?,
            )
            .ok_or(LicenseTermsParseError::InvalidOffsets)?;
        if bytes.len() < header_size {
            return Err(LicenseTermsParseError::TruncatedHeader);
        }

        let mut offsets = Vec::with_capacity(offset_count);
        for index in 0..offset_count {
            offsets.push(read_u32(bytes, 1 + index * 4)? as usize);
        }
        if offsets.first().copied() != Some(header_size)
            || offsets.last().copied() != Some(bytes.len())
            || offsets.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(LicenseTermsParseError::InvalidOffsets);
        }

        let mut blocks = Vec::with_capacity(block_count);
        for (index, range) in offsets.windows(2).enumerate() {
            let stored = bytes
                .get(range[0]..range[1])
                .ok_or(LicenseTermsParseError::InvalidOffsets)?;
            if stored.len() < MWC_HEADER_SIZE || stored.get(..4) != Some(MWC_MAGIC) {
                return Err(LicenseTermsParseError::InvalidMwcMagic { index });
            }
            let decoded_size = read_u32(stored, 4)? as usize;
            let compressed_size = read_u32(stored, 8)? as usize;
            if stored.len() != MWC_HEADER_SIZE.saturating_add(compressed_size) {
                return Err(LicenseTermsParseError::CompressedSizeMismatch { index });
            }
            let mut decoder = ZlibDecoder::new(&stored[MWC_HEADER_SIZE..]);
            let mut decoded = Vec::with_capacity(decoded_size);
            decoder
                .read_to_end(&mut decoded)
                .map_err(|source| LicenseTermsParseError::Decompress { index, source })?;
            if decoded.len() != decoded_size {
                return Err(LicenseTermsParseError::DecodedSizeMismatch {
                    index,
                    expected: decoded_size,
                    actual: decoded.len(),
                });
            }
            if decoded.len() < TERMS_HEADER_SIZE || !decoded.starts_with(b"TERMS") {
                return Err(LicenseTermsParseError::InvalidTermsHeader { index });
            }
            let identifier_bytes = &decoded[..TERMS_IDENTIFIER_SIZE];
            let identifier_end = identifier_bytes
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(identifier_bytes.len());
            let identifier = std::str::from_utf8(&identifier_bytes[..identifier_end])
                .map_err(|_| LicenseTermsParseError::InvalidTermsHeader { index })?
                .to_owned();
            let unit_count = read_u32(&decoded, TERMS_IDENTIFIER_SIZE)? as usize;
            let expected_size = TERMS_HEADER_SIZE
                .checked_add(
                    unit_count
                        .checked_mul(2)
                        .ok_or(LicenseTermsParseError::InvalidTermsSize { index })?,
                )
                .ok_or(LicenseTermsParseError::InvalidTermsSize { index })?;
            if decoded.len() != expected_size {
                return Err(LicenseTermsParseError::InvalidTermsSize { index });
            }
            let units = decoded[TERMS_HEADER_SIZE..]
                .chunks_exact(2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                .collect();
            blocks.push(LicenseTermsBlock {
                index,
                identifier,
                units,
            });
        }

        Ok(Self { blocks })
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, LicenseTermsParseError> {
    let raw = bytes
        .get(offset..offset.saturating_add(4))
        .ok_or(LicenseTermsParseError::UnexpectedEnd)?;
    Ok(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

#[derive(Debug)]
pub enum LicenseTermsParseError {
    TruncatedHeader,
    InvalidOffsets,
    InvalidMwcMagic {
        index: usize,
    },
    CompressedSizeMismatch {
        index: usize,
    },
    Decompress {
        index: usize,
        source: io::Error,
    },
    DecodedSizeMismatch {
        index: usize,
        expected: usize,
        actual: usize,
    },
    InvalidTermsHeader {
        index: usize,
    },
    InvalidTermsSize {
        index: usize,
    },
    UnexpectedEnd,
}

impl fmt::Display for LicenseTermsParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TruncatedHeader => write!(formatter, "LC 헤더가 잘렸습니다."),
            Self::InvalidOffsets => write!(formatter, "LC 블록 오프셋 표가 올바르지 않습니다."),
            Self::InvalidMwcMagic { index } => {
                write!(formatter, "LC 블록 {index}의 MWC 서명이 올바르지 않습니다.")
            }
            Self::CompressedSizeMismatch { index } => {
                write!(
                    formatter,
                    "LC 블록 {index}의 저장 길이가 올바르지 않습니다."
                )
            }
            Self::Decompress { index, source } => {
                write!(
                    formatter,
                    "LC 블록 {index}의 압축을 풀지 못했습니다: {source}"
                )
            }
            Self::DecodedSizeMismatch {
                index,
                expected,
                actual,
            } => write!(
                formatter,
                "LC 블록 {index}의 해제 길이가 다릅니다: 예상 {expected}, 실제 {actual}"
            ),
            Self::InvalidTermsHeader { index } => {
                write!(
                    formatter,
                    "LC 블록 {index}의 TERMS 헤더가 올바르지 않습니다."
                )
            }
            Self::InvalidTermsSize { index } => {
                write!(
                    formatter,
                    "LC 블록 {index}의 TERMS 단위 수가 올바르지 않습니다."
                )
            }
            Self::UnexpectedEnd => write!(formatter, "LC 데이터를 읽는 중 파일 끝에 도달했습니다."),
        }
    }
}

impl Error for LicenseTermsParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Decompress { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write;

    fn fixture() -> Vec<u8> {
        let mut decoded = b"TERMS000020091215\0".to_vec();
        decoded.extend_from_slice(&3u32.to_le_bytes());
        for unit in [0x1234u16, 0xabcd, 0x0042] {
            decoded.extend_from_slice(&unit.to_le_bytes());
        }
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&decoded).expect("write TERMS fixture");
        let compressed = encoder.finish().expect("finish TERMS fixture");
        let header_size = 9u32;
        let file_size = header_size + MWC_HEADER_SIZE as u32 + compressed.len() as u32;
        let mut archive = vec![1];
        archive.extend_from_slice(&header_size.to_le_bytes());
        archive.extend_from_slice(&file_size.to_le_bytes());
        archive.extend_from_slice(MWC_MAGIC);
        archive.extend_from_slice(&(decoded.len() as u32).to_le_bytes());
        archive.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
        archive.extend_from_slice(&compressed);
        archive
    }

    #[test]
    fn parses_compiled_license_terms_blocks() {
        let archive = LicenseTermsArchive::parse(&fixture()).expect("parse LC fixture");
        assert_eq!(archive.blocks.len(), 1);
        assert_eq!(archive.blocks[0].identifier, "TERMS000020091215");
        assert_eq!(archive.blocks[0].units, [0x1234, 0xabcd, 0x0042]);
    }

    #[test]
    fn rejects_an_invalid_offset_table() {
        let mut invalid = fixture();
        invalid[1..5].copy_from_slice(&8u32.to_le_bytes());
        assert!(matches!(
            LicenseTermsArchive::parse(&invalid),
            Err(LicenseTermsParseError::InvalidOffsets)
        ));
    }
}
