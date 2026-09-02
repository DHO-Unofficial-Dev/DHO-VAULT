// SPDX-License-Identifier: MPL-2.0

//! Parser for the small KOVS wrapper used by the client audio files.

use std::error::Error;
use std::fmt;

pub const KOVS_HEADER_SIZE: usize = 32;
const KOVS_XOR_SIZE: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KovsHeader {
    pub payload_size: u32,
    pub loop_start_sample: u32,
}

impl KovsHeader {
    pub fn parse(header: &[u8], file_size: u64) -> Result<Self, KovsParseError> {
        if header.len() < KOVS_HEADER_SIZE {
            return Err(KovsParseError::HeaderTooShort {
                actual: header.len(),
            });
        }
        if &header[..4] != b"KOVS" {
            return Err(KovsParseError::InvalidMagic);
        }
        let payload_size = u32::from_le_bytes(header[4..8].try_into().expect("four bytes"));
        let loop_start_sample = u32::from_le_bytes(header[8..12].try_into().expect("four bytes"));
        let expected = KOVS_HEADER_SIZE as u64 + u64::from(payload_size);
        if file_size != expected {
            return Err(KovsParseError::FileSizeMismatch {
                expected,
                actual: file_size,
            });
        }
        Ok(Self {
            payload_size,
            loop_start_sample,
        })
    }

    pub fn loop_start_seconds(self, sample_rate: u32) -> Option<f64> {
        (sample_rate != 0).then(|| f64::from(self.loop_start_sample) / f64::from(sample_rate))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedKovsAudio {
    pub header: KovsHeader,
    pub ogg: Vec<u8>,
    pub channels: Option<u8>,
    pub sample_rate: Option<u32>,
}

pub fn decode_kovs(bytes: &[u8]) -> Result<DecodedKovsAudio, KovsParseError> {
    let header = KovsHeader::parse(bytes, bytes.len() as u64)?;
    let mut ogg = bytes[KOVS_HEADER_SIZE..].to_vec();
    for (index, byte) in ogg.iter_mut().take(KOVS_XOR_SIZE).enumerate() {
        *byte ^= index as u8;
    }
    if !ogg.starts_with(b"OggS") {
        return Err(KovsParseError::InvalidOggPayload);
    }
    let (channels, sample_rate) = vorbis_identification(&ogg)
        .map_or((None, None), |(channels, sample_rate)| {
            (Some(channels), Some(sample_rate))
        });
    Ok(DecodedKovsAudio {
        header,
        ogg,
        channels,
        sample_rate,
    })
}

fn vorbis_identification(ogg: &[u8]) -> Option<(u8, u32)> {
    if ogg.len() < 28 || &ogg[..4] != b"OggS" {
        return None;
    }
    let segment_count = usize::from(*ogg.get(26)?);
    let packet_offset = 27usize.checked_add(segment_count)?;
    let packet = ogg.get(packet_offset..)?;
    if packet.len() < 16 || packet[0] != 1 || &packet[1..7] != b"vorbis" {
        return None;
    }
    let channels = packet[11];
    let sample_rate = u32::from_le_bytes(packet[12..16].try_into().ok()?);
    (channels != 0 && sample_rate != 0).then_some((channels, sample_rate))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KovsParseError {
    HeaderTooShort { actual: usize },
    InvalidMagic,
    FileSizeMismatch { expected: u64, actual: u64 },
    InvalidOggPayload,
}

impl fmt::Display for KovsParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HeaderTooShort { actual } => write!(
                formatter,
                "KOVS header is shorter than {KOVS_HEADER_SIZE} bytes: {actual}"
            ),
            Self::InvalidMagic => formatter.write_str("audio file does not begin with KOVS"),
            Self::FileSizeMismatch { expected, actual } => {
                write!(
                    formatter,
                    "KOVS file size mismatch: expected {expected}, got {actual}"
                )
            }
            Self::InvalidOggPayload => {
                formatter.write_str("decoded KOVS payload does not begin with OggS")
            }
        }
    }
}

impl Error for KovsParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrapped_ogg() -> Vec<u8> {
        let mut ogg = vec![0_u8; 27 + 1 + 30];
        ogg[..4].copy_from_slice(b"OggS");
        ogg[26] = 1;
        ogg[27] = 30;
        let packet = 28;
        ogg[packet] = 1;
        ogg[packet + 1..packet + 7].copy_from_slice(b"vorbis");
        ogg[packet + 11] = 2;
        ogg[packet + 12..packet + 16].copy_from_slice(&44_100_u32.to_le_bytes());
        let mut encrypted = ogg.clone();
        for (index, byte) in encrypted.iter_mut().take(KOVS_XOR_SIZE).enumerate() {
            *byte ^= index as u8;
        }
        let mut wrapped = vec![0_u8; KOVS_HEADER_SIZE];
        wrapped[..4].copy_from_slice(b"KOVS");
        wrapped[4..8].copy_from_slice(&(encrypted.len() as u32).to_le_bytes());
        wrapped[8..12].copy_from_slice(&22_050_u32.to_le_bytes());
        wrapped.extend(encrypted);
        wrapped
    }

    #[test]
    fn decodes_ogg_and_reads_vorbis_metadata() {
        let decoded = decode_kovs(&wrapped_ogg()).expect("decode KOVS");
        assert!(decoded.ogg.starts_with(b"OggS"));
        assert_eq!(decoded.channels, Some(2));
        assert_eq!(decoded.sample_rate, Some(44_100));
        assert_eq!(decoded.header.loop_start_seconds(44_100), Some(0.5));
    }

    #[test]
    fn rejects_trailing_or_missing_payload_bytes() {
        let mut wrapped = wrapped_ogg();
        wrapped.push(0);
        assert!(matches!(
            decode_kovs(&wrapped),
            Err(KovsParseError::FileSizeMismatch { .. })
        ));
    }
}
