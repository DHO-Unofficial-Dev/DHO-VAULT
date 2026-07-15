// SPDX-License-Identifier: MPL-2.0

//! Pixel conversion and encoding for decoded DHO image resources.

use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};
use std::error::Error;
use std::fmt;

/// An owned image in red, green, blue, alpha byte order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl RgbaImage {
    /// Converts exactly one BGRA pixel per declared image position.
    pub fn from_bgra(width: u32, height: u32, bytes: &[u8]) -> Result<Self, PixelDecodeError> {
        let expected_len = pixel_byte_len(width, height)?;
        if bytes.len() != expected_len {
            return Err(PixelDecodeError::ByteLengthMismatch {
                width,
                height,
                expected_len,
                actual_len: bytes.len(),
            });
        }

        let mut pixels = Vec::with_capacity(expected_len);
        for bgra in bytes.chunks_exact(4) {
            pixels.extend_from_slice(&[bgra[2], bgra[1], bgra[0], bgra[3]]);
        }

        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Encodes this image as a standards-compliant PNG byte stream.
    pub fn encode_png(&self) -> Result<Vec<u8>, PngEncodeError> {
        let mut output = Vec::new();
        PngEncoder::new(&mut output)
            .write_image(
                &self.pixels,
                self.width,
                self.height,
                ExtendedColorType::Rgba8,
            )
            .map_err(|error| PngEncodeError {
                message: error.to_string(),
            })?;
        Ok(output)
    }
}

/// Invalid dimensions or byte counts in a decoded image block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PixelDecodeError {
    DimensionOverflow {
        width: u32,
        height: u32,
    },
    ByteLengthMismatch {
        width: u32,
        height: u32,
        expected_len: usize,
        actual_len: usize,
    },
}

impl fmt::Display for PixelDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DimensionOverflow { width, height } => write!(
                formatter,
                "BGRA image byte length overflows this platform: width={width}, height={height}"
            ),
            Self::ByteLengthMismatch {
                width,
                height,
                expected_len,
                actual_len,
            } => write!(
                formatter,
                "BGRA image byte length mismatch for {width}x{height}: expected {expected_len}, found {actual_len}"
            ),
        }
    }
}

impl Error for PixelDecodeError {}

/// Failure returned by the PNG encoder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PngEncodeError {
    message: String,
}

impl fmt::Display for PngEncodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "failed to encode RGBA image as PNG: {}",
            self.message
        )
    }
}

impl Error for PngEncodeError {}

fn pixel_byte_len(width: u32, height: u32) -> Result<usize, PixelDecodeError> {
    let pixels = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4));
    pixels.ok_or(PixelDecodeError::DimensionOverflow { width, height })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_bgra_to_rgba_and_preserves_alpha() {
        let image = RgbaImage::from_bgra(2, 1, &[0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88])
            .expect("valid BGRA pixels");

        assert_eq!(
            image.pixels(),
            &[0x33, 0x22, 0x11, 0x44, 0x77, 0x66, 0x55, 0x88]
        );
    }

    #[test]
    fn rejects_an_incorrect_byte_length() {
        let error = RgbaImage::from_bgra(2, 2, &[0; 15]).unwrap_err();

        assert_eq!(
            error,
            PixelDecodeError::ByteLengthMismatch {
                width: 2,
                height: 2,
                expected_len: 16,
                actual_len: 15,
            }
        );
    }

    #[test]
    fn rejects_dimensions_whose_byte_count_overflows() {
        let error = RgbaImage::from_bgra(u32::MAX, u32::MAX, &[]).unwrap_err();

        assert_eq!(
            error,
            PixelDecodeError::DimensionOverflow {
                width: u32::MAX,
                height: u32::MAX,
            }
        );
    }

    #[test]
    fn encodes_a_png_with_the_same_rgba_pixels() {
        let image =
            RgbaImage::from_bgra(1, 1, &[0x10, 0x20, 0x30, 0x40]).expect("valid BGRA pixel");

        let png = image.encode_png().expect("encode PNG");
        let decoded = image::load_from_memory_with_format(&png, image::ImageFormat::Png)
            .expect("decode generated PNG")
            .into_rgba8();

        assert_eq!(decoded.dimensions(), (1, 1));
        assert_eq!(decoded.as_raw(), &[0x30, 0x20, 0x10, 0x40]);
    }
}
