// SPDX-License-Identifier: MPL-2.0

//! Parsers for standalone cursor and XFTX texture containers.

use std::error::Error;
use std::fmt;

const CURSOR_WIDTH: u32 = 32;
const CURSOR_HEIGHT: u32 = 32;
const CURSOR_PIXEL_BYTES: usize = 32 * 32 * 4;
const MAX_CURSOR_COUNT: u32 = 1_024;
const XFTX_MAGIC: &[u8; 8] = b"XFTX0200";
const XFTX_HEADER_SIZE: usize = 64;
const MAX_XFTX_DIMENSION: u32 = 8_192;
const FONT_GLYPH_COUNT: usize = 65_536;
const FONT_DIMENSION_TABLE_OFFSET: usize = 4;
const FONT_OFFSET_TABLE_OFFSET: usize = 0x20004;
const FONT_DATA_SENTINEL: u32 = 0x62008;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorImage {
    pub index: u32,
    pub hotspot_x: u16,
    pub hotspot_y: u16,
    pub pixels_bgra: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorArchive {
    pub images: Vec<CursorImage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XftxTexture {
    pub index: u32,
    pub file_offset: usize,
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub name: String,
    pub pixels_bgra: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XftxArchive {
    pub textures: Vec<XftxTexture>,
    pub unresolved_ranges: Vec<(usize, usize)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontGlyph {
    pub code_point: u32,
    pub width: u32,
    pub height: u32,
    pub alpha: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitmapFontArchive {
    pub glyphs: Vec<FontGlyph>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecialImageParseError {
    CursorHeaderTooShort {
        actual_len: usize,
    },
    CursorDeclaredSizeMismatch {
        declared: u32,
        actual: usize,
    },
    CursorCountTooLarge {
        count: u32,
    },
    CursorSizeOverflow,
    CursorPayloadLengthMismatch {
        expected: usize,
        actual: usize,
    },
    XftxNotFound,
    XftxHeaderTruncated {
        offset: usize,
    },
    XftxInvalidDimensions {
        offset: usize,
        width: u32,
        height: u32,
    },
    XftxBlockLengthMismatch {
        offset: usize,
        declared: u32,
        expected: usize,
    },
    XftxBlockTruncated {
        offset: usize,
        required_end: usize,
        actual_len: usize,
    },
    XftxIndexOverflow,
    FontTablesTruncated {
        actual_len: usize,
    },
    FontGlyphDimensionsInvalid {
        code_point: u32,
        width: u32,
        height: u32,
    },
    FontGlyphOffsetOverflow {
        code_point: u32,
    },
    FontGlyphTruncated {
        code_point: u32,
        required_end: usize,
        actual_len: usize,
    },
}

impl fmt::Display for SpecialImageParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CursorHeaderTooShort { actual_len } => write!(
                formatter,
                "cursor header is truncated: need at least 8 bytes, found {actual_len}"
            ),
            Self::CursorDeclaredSizeMismatch { declared, actual } => write!(
                formatter,
                "cursor decoded size mismatch: declared {declared}, actual {actual}"
            ),
            Self::CursorCountTooLarge { count } => {
                write!(formatter, "cursor image count is too large: {count}")
            }
            Self::CursorSizeOverflow => write!(formatter, "cursor size calculation overflowed"),
            Self::CursorPayloadLengthMismatch { expected, actual } => write!(
                formatter,
                "cursor payload length mismatch: expected {expected}, actual {actual}"
            ),
            Self::XftxNotFound => write!(formatter, "no XFTX0200 textures were found"),
            Self::XftxHeaderTruncated { offset } => {
                write!(formatter, "XFTX header at offset {offset} is truncated")
            }
            Self::XftxInvalidDimensions {
                offset,
                width,
                height,
            } => write!(
                formatter,
                "XFTX texture at offset {offset} has invalid dimensions {width}x{height}"
            ),
            Self::XftxBlockLengthMismatch {
                offset,
                declared,
                expected,
            } => write!(
                formatter,
                "XFTX block at offset {offset} declares {declared} bytes, expected {expected}"
            ),
            Self::XftxBlockTruncated {
                offset,
                required_end,
                actual_len,
            } => write!(
                formatter,
                "XFTX block at offset {offset} requires end {required_end}, file length {actual_len}"
            ),
            Self::XftxIndexOverflow => write!(formatter, "XFTX texture index exceeds u32"),
            Self::FontTablesTruncated { actual_len } => write!(
                formatter,
                "bitmap font tables are truncated: need at least 0x60004 bytes, found {actual_len}"
            ),
            Self::FontGlyphDimensionsInvalid {
                code_point,
                width,
                height,
            } => write!(
                formatter,
                "bitmap font glyph U+{code_point:04X} has invalid dimensions {width}x{height}"
            ),
            Self::FontGlyphOffsetOverflow { code_point } => write!(
                formatter,
                "bitmap font glyph U+{code_point:04X} offset calculation overflowed"
            ),
            Self::FontGlyphTruncated {
                code_point,
                required_end,
                actual_len,
            } => write!(
                formatter,
                "bitmap font glyph U+{code_point:04X} needs end {required_end}, file length {actual_len}"
            ),
        }
    }
}

impl Error for SpecialImageParseError {}

impl CursorArchive {
    pub fn parse(decoded: &[u8]) -> Result<Self, SpecialImageParseError> {
        if decoded.len() < 8 {
            return Err(SpecialImageParseError::CursorHeaderTooShort {
                actual_len: decoded.len(),
            });
        }
        let declared = read_u32(decoded, 0);
        if usize::try_from(declared).ok() != Some(decoded.len()) {
            return Err(SpecialImageParseError::CursorDeclaredSizeMismatch {
                declared,
                actual: decoded.len(),
            });
        }
        let count = read_u32(decoded, 4);
        if count > MAX_CURSOR_COUNT {
            return Err(SpecialImageParseError::CursorCountTooLarge { count });
        }
        let header_size = usize::try_from(count)
            .ok()
            .and_then(|count| count.checked_mul(4))
            .and_then(|bytes| 8_usize.checked_add(bytes))
            .ok_or(SpecialImageParseError::CursorSizeOverflow)?;
        let expected = usize::try_from(count)
            .ok()
            .and_then(|count| count.checked_mul(CURSOR_PIXEL_BYTES))
            .and_then(|bytes| header_size.checked_add(bytes))
            .ok_or(SpecialImageParseError::CursorSizeOverflow)?;
        if decoded.len() != expected {
            return Err(SpecialImageParseError::CursorPayloadLengthMismatch {
                expected,
                actual: decoded.len(),
            });
        }
        let mut images = Vec::with_capacity(usize::try_from(count).unwrap_or(0));
        for index in 0..count {
            let index_usize = usize::try_from(index).unwrap_or(usize::MAX);
            let hotspot_offset = 8 + index_usize * 4;
            let pixel_offset = header_size + index_usize * CURSOR_PIXEL_BYTES;
            images.push(CursorImage {
                index,
                hotspot_x: read_u16(decoded, hotspot_offset),
                hotspot_y: read_u16(decoded, hotspot_offset + 2),
                pixels_bgra: decoded[pixel_offset..pixel_offset + CURSOR_PIXEL_BYTES].to_vec(),
            });
        }
        Ok(Self { images })
    }
}

impl XftxArchive {
    pub fn parse(bytes: &[u8]) -> Result<Self, SpecialImageParseError> {
        let mut textures = Vec::new();
        let mut unresolved_ranges = Vec::new();
        let mut cursor = 0_usize;

        while let Some(relative) = find_magic(&bytes[cursor..]) {
            let offset = cursor + relative;
            if offset > cursor {
                unresolved_ranges.push((cursor, offset - cursor));
            }
            let header_end = offset
                .checked_add(XFTX_HEADER_SIZE)
                .ok_or(SpecialImageParseError::XftxHeaderTruncated { offset })?;
            let header = bytes
                .get(offset..header_end)
                .ok_or(SpecialImageParseError::XftxHeaderTruncated { offset })?;
            let declared = read_u32(header, 8);
            let width = u32::from(read_u16(header, 32));
            let height = u32::from(read_u16(header, 34));
            let format = read_u32(header, 36);
            if width == 0
                || height == 0
                || width > MAX_XFTX_DIMENSION
                || height > MAX_XFTX_DIMENSION
            {
                return Err(SpecialImageParseError::XftxInvalidDimensions {
                    offset,
                    width,
                    height,
                });
            }
            let pixel_len = usize::try_from(width)
                .ok()
                .and_then(|width| {
                    usize::try_from(height)
                        .ok()
                        .and_then(|height| width.checked_mul(height))
                })
                .and_then(|pixels| pixels.checked_mul(4))
                .ok_or(SpecialImageParseError::XftxInvalidDimensions {
                    offset,
                    width,
                    height,
                })?;
            let expected = XFTX_HEADER_SIZE.checked_add(pixel_len).ok_or(
                SpecialImageParseError::XftxInvalidDimensions {
                    offset,
                    width,
                    height,
                },
            )?;
            if usize::try_from(declared).ok() != Some(expected) {
                return Err(SpecialImageParseError::XftxBlockLengthMismatch {
                    offset,
                    declared,
                    expected,
                });
            }
            let end =
                offset
                    .checked_add(expected)
                    .ok_or(SpecialImageParseError::XftxBlockTruncated {
                        offset,
                        required_end: usize::MAX,
                        actual_len: bytes.len(),
                    })?;
            let pixels_bgra = bytes
                .get(header_end..end)
                .ok_or(SpecialImageParseError::XftxBlockTruncated {
                    offset,
                    required_end: end,
                    actual_len: bytes.len(),
                })?
                .to_vec();
            let name_end = header[48..64]
                .iter()
                .position(|byte| *byte == 0)
                .map_or(64, |relative| 48 + relative);
            let name = String::from_utf8_lossy(&header[48..name_end]).into_owned();
            textures.push(XftxTexture {
                index: u32::try_from(textures.len())
                    .map_err(|_| SpecialImageParseError::XftxIndexOverflow)?,
                file_offset: offset,
                width,
                height,
                format,
                name,
                pixels_bgra,
            });
            cursor = end;
        }
        if textures.is_empty() {
            return Err(SpecialImageParseError::XftxNotFound);
        }
        if cursor < bytes.len() {
            unresolved_ranges.push((cursor, bytes.len() - cursor));
        }
        Ok(Self {
            textures,
            unresolved_ranges,
        })
    }
}

impl BitmapFontArchive {
    pub fn parse(decoded: &[u8]) -> Result<Self, SpecialImageParseError> {
        let table_end = FONT_OFFSET_TABLE_OFFSET + FONT_GLYPH_COUNT * 4;
        if decoded.len() < table_end {
            return Err(SpecialImageParseError::FontTablesTruncated {
                actual_len: decoded.len(),
            });
        }
        let mut glyphs = Vec::new();
        for code_point in 0..FONT_GLYPH_COUNT {
            let width = u32::from(decoded[FONT_DIMENSION_TABLE_OFFSET + code_point * 2]);
            let height = u32::from(decoded[FONT_DIMENSION_TABLE_OFFSET + code_point * 2 + 1]);
            let offset = read_u32(decoded, FONT_OFFSET_TABLE_OFFSET + code_point * 4);
            if offset == FONT_DATA_SENTINEL {
                continue;
            }
            let code_point = u32::try_from(code_point).unwrap_or(u32::MAX);
            if width == 0 || height == 0 || width > 64 || height > 64 {
                return Err(SpecialImageParseError::FontGlyphDimensionsInvalid {
                    code_point,
                    width,
                    height,
                });
            }
            let pixel_count = usize::try_from(width)
                .ok()
                .and_then(|width| {
                    usize::try_from(height)
                        .ok()
                        .and_then(|height| width.checked_mul(height))
                })
                .ok_or(SpecialImageParseError::FontGlyphOffsetOverflow { code_point })?;
            let packed_len = pixel_count
                .checked_add(3)
                .and_then(|value| value.checked_div(4))
                .ok_or(SpecialImageParseError::FontGlyphOffsetOverflow { code_point })?;
            let offset = usize::try_from(offset)
                .map_err(|_| SpecialImageParseError::FontGlyphOffsetOverflow { code_point })?;
            let end = offset
                .checked_add(packed_len)
                .ok_or(SpecialImageParseError::FontGlyphOffsetOverflow { code_point })?;
            let packed =
                decoded
                    .get(offset..end)
                    .ok_or(SpecialImageParseError::FontGlyphTruncated {
                        code_point,
                        required_end: end,
                        actual_len: decoded.len(),
                    })?;
            let mut alpha = Vec::with_capacity(pixel_count);
            for byte in packed {
                for shift in [6, 4, 2, 0] {
                    if alpha.len() == pixel_count {
                        break;
                    }
                    alpha.push(((byte >> shift) & 0x03) * 85);
                }
            }
            glyphs.push(FontGlyph {
                code_point,
                width,
                height,
                alpha,
            });
        }
        Ok(Self { glyphs })
    }
}

pub const fn cursor_dimensions() -> (u32, u32) {
    (CURSOR_WIDTH, CURSOR_HEIGHT)
}

fn find_magic(bytes: &[u8]) -> Option<usize> {
    bytes
        .windows(XFTX_MAGIC.len())
        .position(|window| window == XFTX_MAGIC)
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cursor_hotspots_and_images() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&4_108_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u32.to_le_bytes());
        bytes.extend_from_slice(&8_u16.to_le_bytes());
        bytes.extend_from_slice(&9_u16.to_le_bytes());
        bytes.extend_from_slice(&vec![0x44; CURSOR_PIXEL_BYTES]);
        let archive = CursorArchive::parse(&bytes).expect("cursor archive");
        assert_eq!(archive.images.len(), 1);
        assert_eq!(
            (archive.images[0].hotspot_x, archive.images[0].hotspot_y),
            (8, 9)
        );
        assert_eq!(archive.images[0].pixels_bgra.len(), CURSOR_PIXEL_BYTES);
    }

    #[test]
    fn finds_xftx_textures_between_unknown_ranges() {
        let mut bytes = vec![0xaa; 3];
        bytes.extend_from_slice(XFTX_MAGIC);
        bytes.extend_from_slice(&80_u32.to_le_bytes());
        bytes.extend_from_slice(&[0; 20]);
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&21_u32.to_le_bytes());
        bytes.extend_from_slice(&[0; 8]);
        bytes.extend_from_slice(b"tex\0");
        bytes.extend_from_slice(&[0; 12]);
        bytes.extend_from_slice(&[0x55; 16]);
        bytes.extend_from_slice(&[0xbb; 2]);
        let archive = XftxArchive::parse(&bytes).expect("XFTX archive");
        assert_eq!(archive.textures.len(), 1);
        assert_eq!(
            (archive.textures[0].width, archive.textures[0].height),
            (2, 2)
        );
        assert_eq!(archive.textures[0].name, "tex");
        assert_eq!(archive.unresolved_ranges, [(0, 3), (83, 2)]);
    }

    #[test]
    fn expands_two_bit_font_alpha() {
        let mut bytes = vec![0; FONT_OFFSET_TABLE_OFFSET + FONT_GLYPH_COUNT * 4];
        for code_point in 0..FONT_GLYPH_COUNT {
            let offset = FONT_OFFSET_TABLE_OFFSET + code_point * 4;
            bytes[offset..offset + 4].copy_from_slice(&FONT_DATA_SENTINEL.to_le_bytes());
        }
        bytes[FONT_DIMENSION_TABLE_OFFSET + 65 * 2] = 2;
        bytes[FONT_DIMENSION_TABLE_OFFSET + 65 * 2 + 1] = 2;
        let glyph_offset = u32::try_from(bytes.len()).expect("fixture offset");
        let table_offset = FONT_OFFSET_TABLE_OFFSET + 65 * 4;
        bytes[table_offset..table_offset + 4].copy_from_slice(&glyph_offset.to_le_bytes());
        bytes.push(0b00_01_10_11);

        let font = BitmapFontArchive::parse(&bytes).expect("bitmap font");
        assert_eq!(font.glyphs.len(), 1);
        assert_eq!(font.glyphs[0].code_point, 65);
        assert_eq!(font.glyphs[0].alpha, [0, 85, 170, 255]);
    }
}
