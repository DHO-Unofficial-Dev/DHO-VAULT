// SPDX-License-Identifier: MPL-2.0

//! Read-only parsing for the typed master tables in `dt000001.bin`.

use flate2::read::ZlibDecoder;
use std::error::Error;
use std::fmt;
use std::io::{self, Read};

const MWC_MAGIC: &[u8; 4] = b"MWC\x1a";
const CONTAINER_POINTER_SIZE: usize = 8;
const MWC_HEADER_SIZE: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DtMasterFieldKind {
    Text,
    U8,
    U16,
    U32,
    Skip(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DtMasterTableSpec {
    pub directory_offset: usize,
    pub fields: &'static [DtMasterFieldKind],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DtMasterValue {
    Text(String),
    U8(u8),
    U16(u16),
    U32(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DtMasterRecord {
    pub id: u32,
    pub values: Vec<DtMasterValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DtMasterData {
    payload: Vec<u8>,
}

impl DtMasterData {
    /// Opens the verified Korean master-data block, falling back to the only block when needed.
    pub fn parse(bytes: &[u8]) -> Result<Self, DtMasterParseError> {
        let block_count = Self::block_count(bytes)?;
        Self::parse_block(bytes, usize::from(block_count > 1))
    }

    /// Returns the number of language-block pointers stored before the first MWC block.
    pub fn block_count(bytes: &[u8]) -> Result<usize, DtMasterParseError> {
        if bytes.len() < CONTAINER_POINTER_SIZE {
            return Err(DtMasterParseError::TruncatedContainer);
        }
        let first_block_offset = read_u32(bytes, 0)? as usize;
        if first_block_offset < CONTAINER_POINTER_SIZE
            || !first_block_offset.is_multiple_of(CONTAINER_POINTER_SIZE)
            || first_block_offset > bytes.len()
        {
            return Err(DtMasterParseError::InvalidDirectory);
        }
        Ok(first_block_offset / CONTAINER_POINTER_SIZE)
    }

    /// Opens one explicitly selected master-data language block.
    pub fn parse_block(bytes: &[u8], block_index: usize) -> Result<Self, DtMasterParseError> {
        let block_count = Self::block_count(bytes)?;
        if block_index >= block_count {
            return Err(DtMasterParseError::BlockIndexOutOfRange {
                block_index,
                block_count,
            });
        }
        let pointer_offset = block_index * CONTAINER_POINTER_SIZE;
        let block_offset = read_u32(bytes, pointer_offset)? as usize;
        let block_length = read_u32(bytes, pointer_offset + 4)? as usize;
        let block_end = block_offset
            .checked_add(block_length)
            .ok_or(DtMasterParseError::InvalidBlockRange)?;
        if block_length < MWC_HEADER_SIZE || block_end > bytes.len() {
            return Err(DtMasterParseError::InvalidBlockRange);
        }
        if bytes.get(block_offset..block_offset + 4) != Some(MWC_MAGIC) {
            return Err(DtMasterParseError::InvalidMwcMagic);
        }
        let decoded_length = read_u32(bytes, block_offset + 4)? as usize;
        let compressed_length = read_u32(bytes, block_offset + 8)? as usize;
        let compressed_start = block_offset + MWC_HEADER_SIZE;
        let compressed_end = compressed_start
            .checked_add(compressed_length)
            .ok_or(DtMasterParseError::InvalidBlockRange)?;
        if compressed_end > block_end {
            return Err(DtMasterParseError::InvalidBlockRange);
        }

        let mut decoder = ZlibDecoder::new(&bytes[compressed_start..compressed_end]);
        let mut payload = Vec::with_capacity(decoded_length);
        decoder
            .read_to_end(&mut payload)
            .map_err(DtMasterParseError::Decompress)?;
        if payload.len() != decoded_length {
            return Err(DtMasterParseError::DecodedLengthMismatch {
                expected: decoded_length,
                actual: payload.len(),
            });
        }
        Ok(Self { payload })
    }

    pub fn read_table(
        &self,
        spec: DtMasterTableSpec,
    ) -> Result<Vec<DtMasterRecord>, DtMasterParseError> {
        self.read_table_with_position(spec)
            .map(|(records, _)| records)
    }

    /// Reads a table and also verifies that its schema consumes the complete table range.
    ///
    /// This is useful for newly discovered tables: a schema that merely decodes the first
    /// few records can otherwise appear valid while becoming misaligned later in the table.
    pub fn read_table_exact(
        &self,
        spec: DtMasterTableSpec,
    ) -> Result<Vec<DtMasterRecord>, DtMasterParseError> {
        let expected_end = self.table_end(spec.directory_offset)?;
        let (records, actual_end) = self.read_table_with_position(spec)?;
        if actual_end != expected_end {
            return Err(DtMasterParseError::TableBoundaryMismatch {
                directory_offset: spec.directory_offset,
                expected_end,
                actual_end,
            });
        }
        Ok(records)
    }

    /// Reads the variable-length action/emotion table used by `dt000001.bin`.
    ///
    /// Each row stores a localized name, two numeric fields, and a counted UTF-16
    /// command such as `/stand`. The final boundary is checked like `read_table_exact`.
    pub fn read_counted_u16_text_table_exact(
        &self,
        directory_offset: usize,
    ) -> Result<Vec<DtMasterRecord>, DtMasterParseError> {
        let table_offset = read_u32(&self.payload, directory_offset)? as usize;
        let expected_end = self.table_end(directory_offset)?;
        let record_count = read_u32(&self.payload, table_offset)? as usize;
        let mut position = table_offset
            .checked_add(4)
            .ok_or(DtMasterParseError::InvalidTable { directory_offset })?;
        let mut records = Vec::with_capacity(record_count);
        for _ in 0..record_count {
            let id = read_u32(&self.payload, position)?;
            position += 4;
            let (name, next_position) = read_master_text(&self.payload, position, id)?;
            position = next_position;
            let first = read_u16(&self.payload, position)?;
            let second = read_u16(&self.payload, position + 2)?;
            let value_count = read_u16(&self.payload, position + 4)? as usize;
            position += 6;
            let mut command_units = Vec::with_capacity(value_count);
            for _ in 0..value_count {
                command_units.push(read_u16(&self.payload, position)?);
                position += 2;
            }
            records.push(DtMasterRecord {
                id,
                values: vec![
                    DtMasterValue::Text(name),
                    DtMasterValue::Text(String::from_utf16_lossy(&command_units)),
                    DtMasterValue::U16(first),
                    DtMasterValue::U16(second),
                ],
            });
        }
        if position != expected_end {
            return Err(DtMasterParseError::TableBoundaryMismatch {
                directory_offset,
                expected_end,
                actual_end: position,
            });
        }
        Ok(records)
    }

    fn table_end(&self, directory_offset: usize) -> Result<usize, DtMasterParseError> {
        let first_table_offset = read_u32(&self.payload, 4)? as usize;
        let next_directory_offset = directory_offset
            .checked_add(8)
            .ok_or(DtMasterParseError::InvalidTable { directory_offset })?;
        if next_directory_offset < first_table_offset {
            Ok(read_u32(&self.payload, next_directory_offset)? as usize)
        } else if next_directory_offset == first_table_offset {
            Ok(self.payload.len())
        } else {
            Err(DtMasterParseError::InvalidTable { directory_offset })
        }
    }

    fn read_table_with_position(
        &self,
        spec: DtMasterTableSpec,
    ) -> Result<(Vec<DtMasterRecord>, usize), DtMasterParseError> {
        let table_offset = read_u32(&self.payload, spec.directory_offset)? as usize;
        let record_count = read_u32(&self.payload, table_offset)? as usize;
        let mut position = table_offset
            .checked_add(4)
            .ok_or(DtMasterParseError::InvalidTable {
                directory_offset: spec.directory_offset,
            })?;
        let mut records = Vec::with_capacity(record_count);

        for _ in 0..record_count {
            let id = read_u32(&self.payload, position)?;
            position += 4;
            let mut values = Vec::new();
            for field in spec.fields {
                match *field {
                    DtMasterFieldKind::Text => {
                        let (text, next_position) = read_master_text(&self.payload, position, id)?;
                        values.push(DtMasterValue::Text(text));
                        position = next_position;
                    }
                    DtMasterFieldKind::U8 => {
                        let value = *self
                            .payload
                            .get(position)
                            .ok_or(DtMasterParseError::UnexpectedEnd)?;
                        position += 1;
                        values.push(DtMasterValue::U8(value));
                    }
                    DtMasterFieldKind::U16 => {
                        values.push(DtMasterValue::U16(read_u16(&self.payload, position)?));
                        position += 2;
                    }
                    DtMasterFieldKind::U32 => {
                        values.push(DtMasterValue::U32(read_u32(&self.payload, position)?));
                        position += 4;
                    }
                    DtMasterFieldKind::Skip(length) => {
                        position = position
                            .checked_add(length)
                            .filter(|end| *end <= self.payload.len())
                            .ok_or(DtMasterParseError::UnexpectedEnd)?;
                    }
                }
            }
            records.push(DtMasterRecord { id, values });
        }
        Ok((records, position))
    }
}

fn read_master_text(
    bytes: &[u8],
    position: usize,
    id: u32,
) -> Result<(String, usize), DtMasterParseError> {
    let encoded_length = read_u16(bytes, position)? as usize;
    let encoded_start = position + 2;
    let encoded_end = encoded_start
        .checked_add(encoded_length)
        .ok_or(DtMasterParseError::InvalidString { id })?;
    let encoded = bytes
        .get(encoded_start..encoded_end)
        .ok_or(DtMasterParseError::InvalidString { id })?;
    Ok((decode_master_string(encoded, id)?, encoded_end))
}

fn decode_master_string(encoded: &[u8], id: u32) -> Result<String, DtMasterParseError> {
    if !encoded.len().is_multiple_of(4) {
        return Err(DtMasterParseError::InvalidEncodedStringLength {
            encoded_length: encoded.len(),
        });
    }
    let mut six_bit = Vec::with_capacity(encoded.len());
    six_bit.extend(encoded.iter().map(|value| (value >> 3) * 7 + (value & 7)));
    let mut decoded = Vec::with_capacity(encoded.len() / 4 * 3);
    for group in six_bit.chunks_exact(4) {
        decoded.push(((group[0] & 0x3f) << 2) | ((group[1] & 0x30) >> 4));
        decoded.push(((group[1] & 0x0f) << 4) | ((group[2] & 0x3c) >> 2));
        decoded.push(((group[2] & 0x03) << 6) | (group[3] & 0x3f));
    }
    let key = id.to_le_bytes();
    for (index, value) in decoded.iter_mut().enumerate() {
        *value ^= key[index & 3];
    }
    let mut units = Vec::new();
    for pair in decoded.chunks_exact(2) {
        let unit = u16::from_le_bytes([pair[0], pair[1]]);
        if unit == 0 {
            break;
        }
        units.push(unit);
    }
    Ok(String::from_utf16_lossy(&units))
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, DtMasterParseError> {
    let raw = bytes
        .get(offset..offset.saturating_add(2))
        .ok_or(DtMasterParseError::UnexpectedEnd)?;
    Ok(u16::from_le_bytes([raw[0], raw[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, DtMasterParseError> {
    let raw = bytes
        .get(offset..offset.saturating_add(4))
        .ok_or(DtMasterParseError::UnexpectedEnd)?;
    Ok(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

#[derive(Debug)]
pub enum DtMasterParseError {
    TruncatedContainer,
    InvalidDirectory,
    InvalidBlockRange,
    InvalidMwcMagic,
    BlockIndexOutOfRange {
        block_index: usize,
        block_count: usize,
    },
    Decompress(io::Error),
    DecodedLengthMismatch {
        expected: usize,
        actual: usize,
    },
    InvalidTable {
        directory_offset: usize,
    },
    TableBoundaryMismatch {
        directory_offset: usize,
        expected_end: usize,
        actual_end: usize,
    },
    InvalidString {
        id: u32,
    },
    InvalidEncodedStringLength {
        encoded_length: usize,
    },
    UnexpectedEnd,
}

impl fmt::Display for DtMasterParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TruncatedContainer => write!(formatter, "DT 마스터 컨테이너 헤더가 잘렸습니다."),
            Self::InvalidDirectory => {
                write!(
                    formatter,
                    "DT 마스터 언어 블록 디렉터리가 올바르지 않습니다."
                )
            }
            Self::InvalidBlockRange => {
                write!(formatter, "DT 마스터 MWC 블록 범위가 올바르지 않습니다.")
            }
            Self::InvalidMwcMagic => {
                write!(formatter, "DT 마스터 MWC 블록 서명이 올바르지 않습니다.")
            }
            Self::BlockIndexOutOfRange {
                block_index,
                block_count,
            } => write!(
                formatter,
                "DT 마스터 언어 블록 {block_index}가 범위를 벗어났습니다: 블록 수 {block_count}"
            ),
            Self::Decompress(error) => {
                write!(formatter, "DT 마스터 블록 압축을 풀지 못했습니다: {error}")
            }
            Self::DecodedLengthMismatch { expected, actual } => write!(
                formatter,
                "DT 마스터 블록 길이가 다릅니다: 예상 {expected}, 실제 {actual}"
            ),
            Self::InvalidTable { directory_offset } => write!(
                formatter,
                "DT 마스터 테이블 디렉터리가 올바르지 않습니다: {directory_offset:#x}"
            ),
            Self::TableBoundaryMismatch {
                directory_offset,
                expected_end,
                actual_end,
            } => write!(
                formatter,
                "DT 마스터 테이블 경계가 일치하지 않습니다: 디렉터리 {directory_offset:#x}, 예상 끝 {expected_end:#x}, 실제 끝 {actual_end:#x}"
            ),
            Self::InvalidString { id } => {
                write!(formatter, "DT 마스터 문자열 {id}가 올바르지 않습니다.")
            }
            Self::InvalidEncodedStringLength { encoded_length } => write!(
                formatter,
                "DT 마스터 문자열 저장 길이가 4바이트 단위가 아닙니다: {encoded_length}"
            ),
            Self::UnexpectedEnd => write!(
                formatter,
                "DT 마스터 데이터를 읽는 중 파일 끝에 도달했습니다."
            ),
        }
    }
}

impl Error for DtMasterParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Decompress(error) => Some(error),
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

    fn push_u16(bytes: &mut Vec<u8>, value: u16) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u32(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn encode_master_string(id: u32, text: &str) -> Vec<u8> {
        let mut plain = text
            .encode_utf16()
            .chain([0])
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        while !plain.len().is_multiple_of(3) {
            plain.push(0);
        }
        let key = id.to_le_bytes();
        for (index, value) in plain.iter_mut().enumerate() {
            *value ^= key[index & 3];
        }
        let mut encoded = Vec::new();
        for chunk in plain.chunks_exact(3) {
            let six = [
                chunk[0] >> 2,
                ((chunk[0] & 3) << 4) | (chunk[1] >> 4),
                ((chunk[1] & 15) << 2) | (chunk[2] >> 6),
                chunk[2] & 63,
            ];
            encoded.extend(six.map(|value| (value / 7) * 8 + value % 7));
        }
        encoded
    }

    fn fixture() -> Vec<u8> {
        let table_offset = 0x40usize;
        let directory_offset = 0x24usize;
        let encoded = encode_master_string(7, "돛 조종");
        let mut payload = vec![0; table_offset];
        payload[directory_offset..directory_offset + 4]
            .copy_from_slice(&(table_offset as u32).to_le_bytes());
        push_u32(&mut payload, 1);
        push_u32(&mut payload, 7);
        push_u16(&mut payload, encoded.len() as u16);
        payload.extend_from_slice(&encoded);
        push_u16(&mut payload, 3);

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&payload).expect("write fixture payload");
        let compressed = encoder.finish().expect("finish fixture payload");
        let block_offset = CONTAINER_POINTER_SIZE * 2;
        let block_length = MWC_HEADER_SIZE + compressed.len();
        let mut container = vec![0; block_offset];
        container[0..4].copy_from_slice(&(block_offset as u32).to_le_bytes());
        container[4..8].copy_from_slice(&(block_length as u32).to_le_bytes());
        container[CONTAINER_POINTER_SIZE..CONTAINER_POINTER_SIZE + 4]
            .copy_from_slice(&(block_offset as u32).to_le_bytes());
        container[CONTAINER_POINTER_SIZE + 4..CONTAINER_POINTER_SIZE + 8]
            .copy_from_slice(&(block_length as u32).to_le_bytes());
        container.extend_from_slice(MWC_MAGIC);
        push_u32(&mut container, payload.len() as u32);
        push_u32(&mut container, compressed.len() as u32);
        container.extend_from_slice(&compressed);
        container
    }

    #[test]
    fn parses_typed_master_records() {
        let fixture = fixture();
        assert_eq!(
            DtMasterData::block_count(&fixture).expect("count blocks"),
            2
        );
        let data = DtMasterData::parse(&fixture).expect("parse master fixture");
        let records = data
            .read_table(DtMasterTableSpec {
                directory_offset: 0x24,
                fields: &[DtMasterFieldKind::Text, DtMasterFieldKind::U16],
            })
            .expect("read master table");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, 7);
        assert_eq!(
            records[0].values,
            [
                DtMasterValue::Text("돛 조종".to_owned()),
                DtMasterValue::U16(3)
            ]
        );
    }

    #[test]
    fn rejects_the_wrong_master_block_signature() {
        let mut bytes = fixture();
        bytes[CONTAINER_POINTER_SIZE * 2] = 0;
        assert!(matches!(
            DtMasterData::parse(&bytes),
            Err(DtMasterParseError::InvalidMwcMagic)
        ));
    }

    #[test]
    fn rejects_an_out_of_range_master_language_block() {
        let bytes = fixture();
        assert!(matches!(
            DtMasterData::parse_block(&bytes, 2),
            Err(DtMasterParseError::BlockIndexOutOfRange {
                block_index: 2,
                block_count: 2
            })
        ));
    }
}
