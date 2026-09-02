// SPDX-License-Identifier: MPL-2.0

//! Read-only parsing for encrypted DT string containers.

use flate2::read::ZlibDecoder;
use std::error::Error;
use std::fmt;
use std::io::{self, Read};

const MWC_MAGIC: &[u8; 4] = b"MWC\x1a";
const CONTAINER_POINTER_SIZE: usize = 8;
const MWC_HEADER_SIZE: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DtTextRecord {
    pub id: u32,
    pub parameters: Vec<u16>,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DtTextTable {
    pub records: Vec<DtTextRecord>,
}

impl DtTextTable {
    /// Parses a DT file whose first referenced MWC block contains a string table.
    pub fn parse(bytes: &[u8]) -> Result<Self, DtTextParseError> {
        Self::parse_block(bytes, 0)
    }

    /// Returns the number of language-block pointers stored before the first MWC block.
    pub fn block_count(bytes: &[u8]) -> Result<usize, DtTextParseError> {
        container_block_count(bytes)
    }

    /// Parses one explicitly selected language block as a string table.
    pub fn parse_block(bytes: &[u8], block_index: usize) -> Result<Self, DtTextParseError> {
        parse_payload(decode_container_payload(bytes, block_index)?)
    }

    /// Parses the fixed-row DT variant used by location and other early tables.
    pub fn parse_fixed_rows(bytes: &[u8]) -> Result<Self, DtTextParseError> {
        Self::parse_fixed_rows_block(bytes, 0)
    }

    /// Parses one explicitly selected language block of the fixed-row DT variant.
    pub fn parse_fixed_rows_block(
        bytes: &[u8],
        block_index: usize,
    ) -> Result<Self, DtTextParseError> {
        parse_fixed_row_payload(decode_container_payload(bytes, block_index)?)
    }

    /// Parses a compact table made of a row count followed by three `u32` values per row.
    pub fn parse_u32_rows(bytes: &[u8]) -> Result<Self, DtTextParseError> {
        Self::parse_u32_rows_block(bytes, 0)
    }

    /// Parses one explicitly selected language block of the compact numeric DT variant.
    pub fn parse_u32_rows_block(
        bytes: &[u8],
        block_index: usize,
    ) -> Result<Self, DtTextParseError> {
        parse_u32_row_payload(decode_container_payload(bytes, block_index)?)
    }
}

fn container_block_count(bytes: &[u8]) -> Result<usize, DtTextParseError> {
    if bytes.len() < CONTAINER_POINTER_SIZE {
        return Err(DtTextParseError::TruncatedContainer);
    }
    let first_block_offset = read_u32(bytes, 0)? as usize;
    if first_block_offset < CONTAINER_POINTER_SIZE
        || !first_block_offset.is_multiple_of(CONTAINER_POINTER_SIZE)
        || first_block_offset > bytes.len()
    {
        return Err(DtTextParseError::InvalidDirectory);
    }
    Ok(first_block_offset / CONTAINER_POINTER_SIZE)
}

fn decode_container_payload(bytes: &[u8], block_index: usize) -> Result<Vec<u8>, DtTextParseError> {
    let block_count = container_block_count(bytes)?;
    if block_index >= block_count {
        return Err(DtTextParseError::BlockIndexOutOfRange {
            block_index,
            block_count,
        });
    }
    let pointer_offset = block_index * CONTAINER_POINTER_SIZE;
    let block_offset = read_u32(bytes, pointer_offset)? as usize;
    let block_length = read_u32(bytes, pointer_offset + 4)? as usize;
    let block_end = block_offset
        .checked_add(block_length)
        .ok_or(DtTextParseError::InvalidBlockRange)?;
    if block_length < MWC_HEADER_SIZE || block_end > bytes.len() {
        return Err(DtTextParseError::InvalidBlockRange);
    }
    if bytes.get(block_offset..block_offset + 4) != Some(MWC_MAGIC) {
        return Err(DtTextParseError::InvalidMwcMagic);
    }

    let decoded_length = read_u32(bytes, block_offset + 4)? as usize;
    let compressed_length = read_u32(bytes, block_offset + 8)? as usize;
    let compressed_start = block_offset + MWC_HEADER_SIZE;
    let compressed_end = compressed_start
        .checked_add(compressed_length)
        .ok_or(DtTextParseError::InvalidBlockRange)?;
    if compressed_end > block_end {
        return Err(DtTextParseError::InvalidBlockRange);
    }

    let mut decoder = ZlibDecoder::new(&bytes[compressed_start..compressed_end]);
    let mut payload = Vec::with_capacity(decoded_length);
    decoder
        .read_to_end(&mut payload)
        .map_err(DtTextParseError::Decompress)?;
    if payload.len() != decoded_length {
        return Err(DtTextParseError::DecodedLengthMismatch {
            expected: decoded_length,
            actual: payload.len(),
        });
    }
    Ok(payload)
}

fn parse_fixed_row_payload(bytes: Vec<u8>) -> Result<DtTextTable, DtTextParseError> {
    let record_count = read_u32(&bytes, 0)? as usize;
    let mut position = 8usize;
    let mut records = Vec::with_capacity(record_count);

    for _ in 0..record_count {
        let id = read_u32(&bytes, position)?;
        position = position
            .checked_add(12)
            .ok_or(DtTextParseError::InvalidRecord { id })?;
        let encoded_length = read_u16(&bytes, position)? as usize;
        position += 2;
        let encoded_end = position
            .checked_add(encoded_length)
            .ok_or(DtTextParseError::InvalidString { id, field_index: 0 })?;
        let encoded = bytes
            .get(position..encoded_end)
            .ok_or(DtTextParseError::InvalidString { id, field_index: 0 })?;
        let mut fields = vec![decode_fixed_row_string(encoded, id)?];
        position = encoded_end;
        for field_index in 1..=3 {
            let end = bytes[position..]
                .iter()
                .position(|byte| *byte == 0)
                .map(|relative| position + relative)
                .ok_or(DtTextParseError::InvalidString { id, field_index })?;
            fields.push(String::from_utf8_lossy(&bytes[position..end]).into_owned());
            position = end + 1;
        }
        position = position
            .checked_add(0x42)
            .filter(|end| *end <= bytes.len())
            .ok_or(DtTextParseError::InvalidRecord { id })?;
        records.push(DtTextRecord {
            id,
            parameters: Vec::new(),
            fields,
        });
    }

    Ok(DtTextTable { records })
}

fn parse_u32_row_payload(bytes: Vec<u8>) -> Result<DtTextTable, DtTextParseError> {
    let record_count = read_u32(&bytes, 0)? as usize;
    let expected_length = 4usize
        .checked_add(
            record_count
                .checked_mul(12)
                .ok_or(DtTextParseError::InvalidDirectory)?,
        )
        .ok_or(DtTextParseError::InvalidDirectory)?;
    if bytes.len() != expected_length {
        return Err(DtTextParseError::InvalidDirectory);
    }
    let mut records = Vec::with_capacity(record_count);
    for row in 0..record_count {
        let position = 4 + row * 12;
        let id = read_u32(&bytes, position)?;
        let first = read_u32(&bytes, position + 4)?;
        let second = read_u32(&bytes, position + 8)?;
        records.push(DtTextRecord {
            id,
            parameters: Vec::new(),
            fields: vec![
                format!("원시 값 1 (u32): {first}"),
                format!("원시 값 2 (u32): {second}"),
            ],
        });
    }
    Ok(DtTextTable { records })
}

fn parse_payload(mut bytes: Vec<u8>) -> Result<DtTextTable, DtTextParseError> {
    let key_length = read_u32(&bytes, 0)? as usize;
    if key_length == 0 {
        return Err(DtTextParseError::EmptyEncryptionKey);
    }
    let directory_header = 4usize
        .checked_add(key_length)
        .ok_or(DtTextParseError::InvalidDirectory)?;
    let directory_start = directory_header
        .checked_add(8)
        .ok_or(DtTextParseError::InvalidDirectory)?;
    if directory_start > bytes.len() {
        return Err(DtTextParseError::InvalidDirectory);
    }

    let record_count = read_u32(&bytes, directory_header)? as usize;
    let data_length_offset = read_u32(&bytes, directory_header + 4)? as usize;
    let directory_end = directory_start
        .checked_add(
            record_count
                .checked_mul(8)
                .ok_or(DtTextParseError::InvalidDirectory)?,
        )
        .ok_or(DtTextParseError::InvalidDirectory)?;
    if directory_end > bytes.len() || data_length_offset < directory_end {
        return Err(DtTextParseError::InvalidDirectory);
    }

    let data_length = read_u32(&bytes, data_length_offset)? as usize;
    let data_start = data_length_offset
        .checked_add(4)
        .ok_or(DtTextParseError::InvalidDataRange)?;
    let data_end = data_start
        .checked_add(data_length)
        .ok_or(DtTextParseError::InvalidDataRange)?;
    if data_end > bytes.len() {
        return Err(DtTextParseError::InvalidDataRange);
    }

    for index in 0..data_length {
        bytes[data_start + index] ^= bytes[4 + index % key_length];
    }

    let mut records = Vec::with_capacity(record_count);
    for row in 0..record_count {
        let entry = directory_start + row * 8;
        let id = read_u32(&bytes, entry)?;
        let relative = read_u32(&bytes, entry + 4)? as usize;
        let record_start = data_start
            .checked_add(relative)
            .ok_or(DtTextParseError::InvalidRecord { id })?;
        if record_start >= data_end {
            return Err(DtTextParseError::InvalidRecord { id });
        }

        let field_count = read_u16(&bytes, record_start + 2)? as usize;
        let descriptor_start = record_start + 4;
        let descriptor_end = descriptor_start
            .checked_add(
                field_count
                    .checked_mul(6)
                    .ok_or(DtTextParseError::InvalidRecord { id })?,
            )
            .ok_or(DtTextParseError::InvalidRecord { id })?;
        if descriptor_end > data_end {
            return Err(DtTextParseError::InvalidRecord { id });
        }

        let mut parameters = Vec::with_capacity(field_count);
        let mut fields = Vec::with_capacity(field_count);
        for field_index in 0..field_count {
            let descriptor = descriptor_start + field_index * 6;
            parameters.push(read_u16(&bytes, descriptor)?);
            let string_relative = read_u16(&bytes, descriptor + 2)? as usize;
            let encoded_length = read_u16(&bytes, descriptor + 4)? as usize;
            if !encoded_length.is_multiple_of(2) {
                return Err(DtTextParseError::InvalidString { id, field_index });
            }
            let string_start = record_start
                .checked_add(string_relative)
                .ok_or(DtTextParseError::InvalidString { id, field_index })?;
            let string_end = string_start
                .checked_add(encoded_length)
                .ok_or(DtTextParseError::InvalidString { id, field_index })?;
            if string_end > data_end {
                return Err(DtTextParseError::InvalidString { id, field_index });
            }

            let mut units = Vec::with_capacity(encoded_length / 2);
            for (unit_index, pair) in bytes[string_start..string_end].chunks_exact(2).enumerate() {
                let encrypted = u16::from_be_bytes([pair[0], pair[1]]) as u32;
                let key = id.wrapping_add(unit_index as u32);
                units.push((encrypted ^ key) as u16);
            }
            while units.last() == Some(&0) {
                units.pop();
            }
            fields.push(String::from_utf16_lossy(&units));
        }
        records.push(DtTextRecord {
            id,
            parameters,
            fields,
        });
    }

    Ok(DtTextTable { records })
}

fn decode_fixed_row_string(encoded: &[u8], id: u32) -> Result<String, DtTextParseError> {
    if !encoded.len().is_multiple_of(4) {
        return Err(DtTextParseError::InvalidString { id, field_index: 0 });
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
    let units = decoded
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .take_while(|unit| *unit != 0)
        .collect::<Vec<_>>();
    Ok(String::from_utf16_lossy(&units))
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, DtTextParseError> {
    let raw = bytes
        .get(offset..offset.saturating_add(2))
        .ok_or(DtTextParseError::UnexpectedEnd)?;
    Ok(u16::from_le_bytes([raw[0], raw[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, DtTextParseError> {
    let raw = bytes
        .get(offset..offset.saturating_add(4))
        .ok_or(DtTextParseError::UnexpectedEnd)?;
    Ok(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

#[derive(Debug)]
pub enum DtTextParseError {
    TruncatedContainer,
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
    EmptyEncryptionKey,
    InvalidDirectory,
    InvalidDataRange,
    InvalidRecord {
        id: u32,
    },
    InvalidString {
        id: u32,
        field_index: usize,
    },
    UnexpectedEnd,
}

impl fmt::Display for DtTextParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TruncatedContainer => write!(formatter, "DT 컨테이너 헤더가 잘렸습니다."),
            Self::InvalidBlockRange => write!(formatter, "DT MWC 블록 범위가 올바르지 않습니다."),
            Self::InvalidMwcMagic => write!(formatter, "DT MWC 블록 서명이 올바르지 않습니다."),
            Self::BlockIndexOutOfRange {
                block_index,
                block_count,
            } => write!(
                formatter,
                "DT 언어 블록 {block_index}가 범위를 벗어났습니다: 블록 수 {block_count}"
            ),
            Self::Decompress(error) => {
                write!(formatter, "DT 문자열 블록 압축을 풀지 못했습니다: {error}")
            }
            Self::DecodedLengthMismatch { expected, actual } => write!(
                formatter,
                "DT 문자열 블록 길이가 다릅니다: 예상 {expected}, 실제 {actual}"
            ),
            Self::EmptyEncryptionKey => write!(formatter, "DT 문자열 암호화 키가 비어 있습니다."),
            Self::InvalidDirectory => write!(formatter, "DT 문자열 디렉터리가 올바르지 않습니다."),
            Self::InvalidDataRange => {
                write!(formatter, "DT 문자열 데이터 범위가 올바르지 않습니다.")
            }
            Self::InvalidRecord { id } => {
                write!(formatter, "DT 문자열 레코드 {id}가 올바르지 않습니다.")
            }
            Self::InvalidString { id, field_index } => write!(
                formatter,
                "DT 문자열 레코드 {id}의 필드 {field_index}가 올바르지 않습니다."
            ),
            Self::UnexpectedEnd => write!(
                formatter,
                "DT 문자열 데이터를 읽는 중 파일 끝에 도달했습니다."
            ),
        }
    }
}

impl Error for DtTextParseError {
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

    fn encoded_string(id: u32, text: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        for (index, unit) in text.encode_utf16().chain([0]).enumerate() {
            let encrypted = (unit as u32 ^ id.wrapping_add(index as u32)) as u16;
            bytes.extend_from_slice(&encrypted.to_be_bytes());
        }
        bytes
    }

    fn encoded_fixed_string(id: u32, text: &str) -> Vec<u8> {
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

    fn fixture_with_first_text(first_text: &str) -> Vec<u8> {
        let key = b"(C)2005-2006 KOEI Co., Ltd. All rights reserved.\0";
        assert_eq!(key.len(), 49);
        let first = encoded_string(1001, first_text);
        let second_a = encoded_string(2002, "두 번째");
        let second_b = encoded_string(2002, "설명");

        let mut data = Vec::new();
        let first_offset = data.len() as u32;
        push_u16(&mut data, 0);
        push_u16(&mut data, 1);
        push_u16(&mut data, 7);
        push_u16(&mut data, 10);
        push_u16(&mut data, first.len() as u16);
        data.extend_from_slice(&first);

        let second_offset = data.len() as u32;
        push_u16(&mut data, 0);
        push_u16(&mut data, 2);
        let strings_offset = 16u16;
        push_u16(&mut data, 0);
        push_u16(&mut data, strings_offset);
        push_u16(&mut data, second_a.len() as u16);
        push_u16(&mut data, 1);
        push_u16(&mut data, strings_offset + second_a.len() as u16);
        push_u16(&mut data, second_b.len() as u16);
        data.extend_from_slice(&second_a);
        data.extend_from_slice(&second_b);

        for (index, byte) in data.iter_mut().enumerate() {
            *byte ^= key[index % key.len()];
        }

        let directory_header = 4 + key.len();
        let data_length_offset = directory_header + 8 + 16;
        let mut payload = Vec::new();
        push_u32(&mut payload, key.len() as u32);
        payload.extend_from_slice(key);
        push_u32(&mut payload, 2);
        push_u32(&mut payload, data_length_offset as u32);
        push_u32(&mut payload, 1001);
        push_u32(&mut payload, first_offset);
        push_u32(&mut payload, 2002);
        push_u32(&mut payload, second_offset);
        push_u32(&mut payload, data.len() as u32);
        payload.extend_from_slice(&data);

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&payload).expect("write fixture payload");
        let compressed = encoder.finish().expect("finish fixture payload");
        let block_offset = 8u32;
        let block_length = (MWC_HEADER_SIZE + compressed.len()) as u32;
        let mut container = Vec::new();
        push_u32(&mut container, block_offset);
        push_u32(&mut container, block_length);
        container.extend_from_slice(MWC_MAGIC);
        push_u32(&mut container, payload.len() as u32);
        push_u32(&mut container, compressed.len() as u32);
        container.extend_from_slice(&compressed);
        container
    }

    fn fixture() -> Vec<u8> {
        fixture_with_first_text("첫 번째 임무")
    }

    fn multi_block_fixture() -> Vec<u8> {
        let first = fixture_with_first_text("日本語原文");
        let second = fixture_with_first_text("한국어 현지화");
        let first_block = &first[CONTAINER_POINTER_SIZE..];
        let second_block = &second[CONTAINER_POINTER_SIZE..];
        let header_size = CONTAINER_POINTER_SIZE * 2;
        let second_offset = header_size + first_block.len();
        let mut container = Vec::new();
        push_u32(&mut container, header_size as u32);
        push_u32(&mut container, first_block.len() as u32);
        push_u32(&mut container, second_offset as u32);
        push_u32(&mut container, second_block.len() as u32);
        container.extend_from_slice(first_block);
        container.extend_from_slice(second_block);
        container
    }

    fn fixed_row_fixture() -> Vec<u8> {
        let id = 42u32;
        let encoded = encoded_fixed_string(id, "리스본");
        let mut payload = Vec::new();
        push_u32(&mut payload, 1);
        push_u32(&mut payload, 0);
        push_u32(&mut payload, id);
        push_u32(&mut payload, 0);
        push_u32(&mut payload, 0);
        push_u16(&mut payload, encoded.len() as u16);
        payload.extend_from_slice(&encoded);
        payload.extend_from_slice(b"internal\0region\0type\0");
        payload.extend_from_slice(&[0; 0x42]);

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&payload)
            .expect("write fixed row payload");
        let compressed = encoder.finish().expect("finish fixed row payload");
        let block_offset = 8u32;
        let block_length = (MWC_HEADER_SIZE + compressed.len()) as u32;
        let mut container = Vec::new();
        push_u32(&mut container, block_offset);
        push_u32(&mut container, block_length);
        container.extend_from_slice(MWC_MAGIC);
        push_u32(&mut container, payload.len() as u32);
        push_u32(&mut container, compressed.len() as u32);
        container.extend_from_slice(&compressed);
        container
    }

    fn u32_row_fixture() -> Vec<u8> {
        let mut payload = Vec::new();
        push_u32(&mut payload, 2);
        for values in [[1, 0, 10], [2, 3, u32::MAX]] {
            for value in values {
                push_u32(&mut payload, value);
            }
        }
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&payload).expect("write u32 row payload");
        let compressed = encoder.finish().expect("finish u32 row payload");
        let block_offset = 8u32;
        let block_length = (MWC_HEADER_SIZE + compressed.len()) as u32;
        let mut container = Vec::new();
        push_u32(&mut container, block_offset);
        push_u32(&mut container, block_length);
        container.extend_from_slice(MWC_MAGIC);
        push_u32(&mut container, payload.len() as u32);
        push_u32(&mut container, compressed.len() as u32);
        container.extend_from_slice(&compressed);
        container
    }

    #[test]
    fn parses_encrypted_multi_field_records() {
        let table = DtTextTable::parse(&fixture()).expect("parse text fixture");
        assert_eq!(table.records.len(), 2);
        assert_eq!(table.records[0].id, 1001);
        assert_eq!(table.records[0].parameters, [7]);
        assert_eq!(table.records[0].fields, ["첫 번째 임무"]);
        assert_eq!(table.records[1].id, 2002);
        assert_eq!(table.records[1].parameters, [0, 1]);
        assert_eq!(table.records[1].fields, ["두 번째", "설명"]);
    }

    #[test]
    fn selects_a_language_block_by_pointer_index() {
        let fixture = multi_block_fixture();
        assert_eq!(DtTextTable::block_count(&fixture).expect("count blocks"), 2);
        let original = DtTextTable::parse_block(&fixture, 0).expect("parse original block");
        let localized =
            DtTextTable::parse_block(&fixture, 1).expect("parse localized language block");
        assert_eq!(original.records[0].fields, ["日本語原文"]);
        assert_eq!(localized.records[0].fields, ["한국어 현지화"]);
        assert!(matches!(
            DtTextTable::parse_block(&fixture, 2),
            Err(DtTextParseError::BlockIndexOutOfRange {
                block_index: 2,
                block_count: 2
            })
        ));
    }

    #[test]
    fn parses_fixed_row_text_records() {
        let table = DtTextTable::parse_fixed_rows(&fixed_row_fixture())
            .expect("parse fixed row text fixture");
        assert_eq!(table.records.len(), 1);
        assert_eq!(table.records[0].id, 42);
        assert_eq!(
            table.records[0].fields,
            ["리스본", "internal", "region", "type"]
        );
    }

    #[test]
    fn parses_compact_u32_rows() {
        let table =
            DtTextTable::parse_u32_rows(&u32_row_fixture()).expect("parse compact u32 row fixture");
        assert_eq!(table.records.len(), 2);
        assert_eq!(table.records[0].id, 1);
        assert_eq!(
            table.records[1].fields,
            ["원시 값 1 (u32): 3", "원시 값 2 (u32): 4294967295"]
        );
    }

    #[test]
    fn rejects_truncated_and_invalid_containers() {
        assert!(matches!(
            DtTextTable::parse(&[0; 4]),
            Err(DtTextParseError::TruncatedContainer)
        ));
        let mut invalid = fixture();
        invalid[8] = 0;
        assert!(matches!(
            DtTextTable::parse(&invalid),
            Err(DtTextParseError::InvalidMwcMagic)
        ));
    }
}
