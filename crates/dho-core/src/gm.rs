// SPDX-License-Identifier: MPL-2.0

//! Parser for the decoded payload stored in `gmNNNNNN.bin` UI atlas files.

use std::error::Error;
use std::fmt;

const HEADER_SIZE: usize = 12;
const SPRITE_RECORD_SIZE: usize = 28;
const IMAGE_HEADER_SIZE: usize = 20;
const MAX_IMAGE_COUNT: u32 = 16_384;
const MAX_DIMENSION: u32 = 8_192;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GmAtlasImage {
    pub image_index: u32,
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub variant: u32,
    pub pixels_bgra: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GmAtlasSprite {
    pub sprite_index: u32,
    pub image_index: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub u_min: f32,
    pub v_min: f32,
    pub u_max: f32,
    pub v_max: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GmAtlas {
    pub sprite_count: u32,
    pub images: Vec<GmAtlasImage>,
    pub sprites: Vec<GmAtlasSprite>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GmAtlasParseError {
    HeaderTooShort {
        actual_len: usize,
    },
    CountTooLarge {
        image_count: u32,
    },
    OffsetOverflow,
    InvalidImageDataOffset {
        declared: u32,
        expected: usize,
    },
    ImageHeaderTruncated {
        image_index: u32,
    },
    InvalidDimensions {
        image_index: u32,
        width: u32,
        height: u32,
    },
    PixelLengthMismatch {
        image_index: u32,
        declared: u32,
        repeated: u32,
        expected: usize,
    },
    PixelDataTruncated {
        image_index: u32,
        required_end: usize,
        actual_len: usize,
    },
    TrailingBytes {
        consumed: usize,
        actual_len: usize,
    },
    InvalidSpriteImage {
        sprite_index: u32,
        image_index: u32,
        image_count: u32,
    },
    InvalidSpriteDimensions {
        sprite_index: u32,
        width: u32,
        height: u32,
    },
    InvalidSpriteCoordinates {
        sprite_index: u32,
        image_index: u32,
    },
}

impl fmt::Display for GmAtlasParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HeaderTooShort { actual_len } => write!(
                formatter,
                "GM atlas header is truncated: need {HEADER_SIZE} bytes, found {actual_len}"
            ),
            Self::CountTooLarge { image_count } => {
                write!(
                    formatter,
                    "GM atlas image count is too large: {image_count}"
                )
            }
            Self::OffsetOverflow => write!(formatter, "GM atlas offset calculation overflowed"),
            Self::InvalidImageDataOffset { declared, expected } => write!(
                formatter,
                "GM atlas image offset mismatch: declared {declared}, expected {expected}"
            ),
            Self::ImageHeaderTruncated { image_index } => {
                write!(
                    formatter,
                    "GM atlas image {image_index} header is truncated"
                )
            }
            Self::InvalidDimensions {
                image_index,
                width,
                height,
            } => write!(
                formatter,
                "GM atlas image {image_index} has invalid dimensions {width}x{height}"
            ),
            Self::PixelLengthMismatch {
                image_index,
                declared,
                repeated,
                expected,
            } => write!(
                formatter,
                "GM atlas image {image_index} pixel length mismatch: {declared}/{repeated}, expected {expected}"
            ),
            Self::PixelDataTruncated {
                image_index,
                required_end,
                actual_len,
            } => write!(
                formatter,
                "GM atlas image {image_index} is truncated: need end {required_end}, file length {actual_len}"
            ),
            Self::TrailingBytes {
                consumed,
                actual_len,
            } => write!(
                formatter,
                "GM atlas has trailing bytes: consumed {consumed}, file length {actual_len}"
            ),
            Self::InvalidSpriteImage {
                sprite_index,
                image_index,
                image_count,
            } => write!(
                formatter,
                "GM sprite {sprite_index} references image {image_index}, but only {image_count} images exist"
            ),
            Self::InvalidSpriteDimensions {
                sprite_index,
                width,
                height,
            } => write!(
                formatter,
                "GM sprite {sprite_index} has invalid dimensions {width}x{height}"
            ),
            Self::InvalidSpriteCoordinates {
                sprite_index,
                image_index,
            } => write!(
                formatter,
                "GM sprite {sprite_index} is outside atlas image {image_index}"
            ),
        }
    }
}

impl Error for GmAtlasParseError {}

impl GmAtlas {
    pub fn parse(decoded: &[u8]) -> Result<Self, GmAtlasParseError> {
        if decoded.len() < HEADER_SIZE {
            return Err(GmAtlasParseError::HeaderTooShort {
                actual_len: decoded.len(),
            });
        }
        let sprite_count = read_u32(decoded, 0);
        let image_count = read_u32(decoded, 4);
        let image_data_offset = read_u32(decoded, 8);
        if image_count > MAX_IMAGE_COUNT {
            return Err(GmAtlasParseError::CountTooLarge { image_count });
        }
        let expected_offset = usize::try_from(sprite_count)
            .ok()
            .and_then(|count| count.checked_mul(SPRITE_RECORD_SIZE))
            .and_then(|bytes| HEADER_SIZE.checked_add(bytes))
            .ok_or(GmAtlasParseError::OffsetOverflow)?;
        if usize::try_from(image_data_offset).ok() != Some(expected_offset) {
            return Err(GmAtlasParseError::InvalidImageDataOffset {
                declared: image_data_offset,
                expected: expected_offset,
            });
        }

        let mut raw_sprites = Vec::with_capacity(usize::try_from(sprite_count).unwrap_or(0));
        for sprite_index in 0..sprite_count {
            let offset = HEADER_SIZE
                + usize::try_from(sprite_index)
                    .map_err(|_| GmAtlasParseError::OffsetOverflow)?
                    .checked_mul(SPRITE_RECORD_SIZE)
                    .ok_or(GmAtlasParseError::OffsetOverflow)?;
            let record = decoded
                .get(offset..offset + SPRITE_RECORD_SIZE)
                .ok_or(GmAtlasParseError::OffsetOverflow)?;
            raw_sprites.push((
                sprite_index,
                read_u32(record, 0),
                read_f32(record, 4),
                read_f32(record, 8),
                read_f32(record, 12),
                read_f32(record, 16),
                read_u32(record, 20),
                read_u32(record, 24),
            ));
        }

        let mut cursor = expected_offset;
        let mut images = Vec::with_capacity(usize::try_from(image_count).unwrap_or(0));
        for image_index in 0..image_count {
            let header_end = cursor
                .checked_add(IMAGE_HEADER_SIZE)
                .ok_or(GmAtlasParseError::OffsetOverflow)?;
            let header = decoded
                .get(cursor..header_end)
                .ok_or(GmAtlasParseError::ImageHeaderTruncated { image_index })?;
            let width = u32::from(read_u16(header, 0));
            let height = u32::from(read_u16(header, 2));
            let format = read_u32(header, 4);
            let variant = read_u32(header, 8);
            let declared = read_u32(header, 12);
            let repeated = read_u32(header, 16);
            if width == 0 || height == 0 || width > MAX_DIMENSION || height > MAX_DIMENSION {
                return Err(GmAtlasParseError::InvalidDimensions {
                    image_index,
                    width,
                    height,
                });
            }
            let expected = usize::try_from(width)
                .ok()
                .and_then(|width| {
                    usize::try_from(height)
                        .ok()
                        .and_then(|height| width.checked_mul(height))
                })
                .and_then(|pixels| pixels.checked_mul(4))
                .ok_or(GmAtlasParseError::OffsetOverflow)?;
            if usize::try_from(declared).ok() != Some(expected) || declared != repeated {
                return Err(GmAtlasParseError::PixelLengthMismatch {
                    image_index,
                    declared,
                    repeated,
                    expected,
                });
            }
            let pixel_end = header_end
                .checked_add(expected)
                .ok_or(GmAtlasParseError::OffsetOverflow)?;
            let pixels_bgra = decoded
                .get(header_end..pixel_end)
                .ok_or(GmAtlasParseError::PixelDataTruncated {
                    image_index,
                    required_end: pixel_end,
                    actual_len: decoded.len(),
                })?
                .to_vec();
            images.push(GmAtlasImage {
                image_index,
                width,
                height,
                format,
                variant,
                pixels_bgra,
            });
            cursor = pixel_end;
        }
        if cursor != decoded.len() {
            return Err(GmAtlasParseError::TrailingBytes {
                consumed: cursor,
                actual_len: decoded.len(),
            });
        }
        let mut sprites = Vec::with_capacity(raw_sprites.len());
        for (sprite_index, image_index, u_min, v_min, u_max, v_max, width, height) in raw_sprites {
            let image = images
                .get(usize::try_from(image_index).unwrap_or(usize::MAX))
                .ok_or(GmAtlasParseError::InvalidSpriteImage {
                    sprite_index,
                    image_index,
                    image_count,
                })?;
            if width == 0 || height == 0 {
                return Err(GmAtlasParseError::InvalidSpriteDimensions {
                    sprite_index,
                    width,
                    height,
                });
            }
            let valid_uv = [u_min, v_min, u_max, v_max]
                .iter()
                .all(|value| value.is_finite())
                && (0.0..=1.0).contains(&u_min)
                && (0.0..=1.0).contains(&v_min);
            let x = (u_min * image.width as f32).floor();
            let y = (v_min * image.height as f32).floor();
            let valid_bounds = valid_uv
                && x >= 0.0
                && y >= 0.0
                && u_max >= u_min
                && v_max >= v_min
                && x <= u32::MAX as f32
                && y <= u32::MAX as f32;
            if !valid_bounds {
                return Err(GmAtlasParseError::InvalidSpriteCoordinates {
                    sprite_index,
                    image_index,
                });
            }
            let x = x as u32;
            let y = y as u32;
            if x.checked_add(width).is_none_or(|right| right > image.width)
                || y.checked_add(height)
                    .is_none_or(|bottom| bottom > image.height)
            {
                return Err(GmAtlasParseError::InvalidSpriteCoordinates {
                    sprite_index,
                    image_index,
                });
            }
            sprites.push(GmAtlasSprite {
                sprite_index,
                image_index,
                x,
                y,
                width,
                height,
                u_min,
                v_min,
                u_max,
                v_max,
            });
        }
        Ok(Self {
            sprite_count,
            images,
            sprites,
        })
    }
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

fn read_f32(bytes: &[u8], offset: usize) -> f32 {
    f32::from_bits(read_u32(bytes, offset))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_one_bgra_image_after_the_sprite_table() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u32.to_le_bytes());
        bytes.extend_from_slice(&40_u32.to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(&0.25_f32.to_le_bytes());
        bytes.extend_from_slice(&0.5_f32.to_le_bytes());
        bytes.extend_from_slice(&1.0_f32.to_le_bytes());
        bytes.extend_from_slice(&1.0_f32.to_le_bytes());
        bytes.extend_from_slice(&2_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u32.to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&21_u32.to_le_bytes());
        bytes.extend_from_slice(&7_u32.to_le_bytes());
        bytes.extend_from_slice(&8_u32.to_le_bytes());
        bytes.extend_from_slice(&8_u32.to_le_bytes());
        bytes.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);

        let atlas = GmAtlas::parse(&bytes).expect("valid GM atlas");
        assert_eq!(atlas.sprite_count, 1);
        assert_eq!(atlas.sprites.len(), 1);
        assert_eq!((atlas.sprites[0].x, atlas.sprites[0].y), (0, 0));
        assert_eq!((atlas.sprites[0].width, atlas.sprites[0].height), (2, 1));
        assert_eq!(atlas.images.len(), 1);
        assert_eq!((atlas.images[0].width, atlas.images[0].height), (2, 1));
        assert_eq!(atlas.images[0].format, 21);
        assert_eq!(atlas.images[0].variant, 7);
        assert_eq!(atlas.images[0].pixels_bgra, [1, 2, 3, 4, 5, 6, 7, 8]);
    }
}
