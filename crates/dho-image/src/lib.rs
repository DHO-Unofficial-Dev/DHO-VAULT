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

    /// Joins row-major RGBA tiles whose last row or column may be smaller.
    pub fn assemble_grid(
        tiles: &[Self],
        columns: u32,
        rows: u32,
        output_width: u32,
        output_height: u32,
        max_output_size: usize,
    ) -> Result<Self, ImageAssemblyError> {
        if columns == 0 || rows == 0 {
            return Err(ImageAssemblyError::EmptyGrid { columns, rows });
        }

        let expected_tile_count = usize::try_from(columns)
            .ok()
            .and_then(|columns| {
                usize::try_from(rows)
                    .ok()
                    .and_then(|rows| columns.checked_mul(rows))
            })
            .ok_or(ImageAssemblyError::GridOverflow { columns, rows })?;
        if tiles.len() != expected_tile_count {
            return Err(ImageAssemblyError::TileCountMismatch {
                columns,
                rows,
                expected: expected_tile_count,
                actual: tiles.len(),
            });
        }

        let columns_usize = usize::try_from(columns)
            .map_err(|_| ImageAssemblyError::GridOverflow { columns, rows })?;
        let rows_usize = usize::try_from(rows)
            .map_err(|_| ImageAssemblyError::GridOverflow { columns, rows })?;
        let column_widths = tiles[..columns_usize]
            .iter()
            .map(|tile| tile.width)
            .collect::<Vec<_>>();
        let row_heights = (0..rows_usize)
            .map(|row| tiles[row * columns_usize].height)
            .collect::<Vec<_>>();

        for (tile_index, tile) in tiles.iter().enumerate() {
            let row = tile_index / columns_usize;
            let column = tile_index % columns_usize;
            if tile.width != column_widths[column] {
                return Err(ImageAssemblyError::ColumnWidthMismatch {
                    tile_index,
                    column: column as u32,
                    expected: column_widths[column],
                    actual: tile.width,
                });
            }
            if tile.height != row_heights[row] {
                return Err(ImageAssemblyError::RowHeightMismatch {
                    tile_index,
                    row: row as u32,
                    expected: row_heights[row],
                    actual: tile.height,
                });
            }
        }

        let actual_width =
            checked_dimension_sum(&column_widths).ok_or(ImageAssemblyError::DimensionOverflow {
                output_width,
                output_height,
            })?;
        let actual_height =
            checked_dimension_sum(&row_heights).ok_or(ImageAssemblyError::DimensionOverflow {
                output_width,
                output_height,
            })?;
        if (actual_width, actual_height) != (output_width, output_height) {
            return Err(ImageAssemblyError::OutputDimensionMismatch {
                expected: (output_width, output_height),
                actual: (actual_width, actual_height),
            });
        }

        let output_len = pixel_byte_len(output_width, output_height).map_err(|_| {
            ImageAssemblyError::DimensionOverflow {
                output_width,
                output_height,
            }
        })?;
        if output_len > max_output_size {
            return Err(ImageAssemblyError::OutputTooLarge {
                required: output_len,
                maximum: max_output_size,
            });
        }

        let mut pixels = vec![0; output_len];
        let output_stride =
            usize::try_from(output_width).expect("validated output width fits usize") * 4;
        let mut y = 0_usize;
        for row in 0..rows_usize {
            let mut x = 0_usize;
            for column in 0..columns_usize {
                let tile = &tiles[row * columns_usize + column];
                let tile_width = usize::try_from(tile.width).expect("tile width fits usize");
                let tile_height = usize::try_from(tile.height).expect("tile height fits usize");
                let tile_stride = tile_width * 4;
                for tile_y in 0..tile_height {
                    let source_start = tile_y * tile_stride;
                    let target_start = (y + tile_y) * output_stride + x * 4;
                    pixels[target_start..target_start + tile_stride]
                        .copy_from_slice(&tile.pixels[source_start..source_start + tile_stride]);
                }
                x += tile_width;
            }
            y += usize::try_from(row_heights[row]).expect("row height fits usize");
        }

        Ok(Self {
            width: output_width,
            height: output_height,
            pixels,
        })
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

/// A structural or resource-limit failure while joining decoded tiles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageAssemblyError {
    EmptyGrid {
        columns: u32,
        rows: u32,
    },
    GridOverflow {
        columns: u32,
        rows: u32,
    },
    TileCountMismatch {
        columns: u32,
        rows: u32,
        expected: usize,
        actual: usize,
    },
    ColumnWidthMismatch {
        tile_index: usize,
        column: u32,
        expected: u32,
        actual: u32,
    },
    RowHeightMismatch {
        tile_index: usize,
        row: u32,
        expected: u32,
        actual: u32,
    },
    OutputDimensionMismatch {
        expected: (u32, u32),
        actual: (u32, u32),
    },
    DimensionOverflow {
        output_width: u32,
        output_height: u32,
    },
    OutputTooLarge {
        required: usize,
        maximum: usize,
    },
}

impl fmt::Display for ImageAssemblyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyGrid { columns, rows } => {
                write!(formatter, "image grid must not be empty: {columns}x{rows}")
            }
            Self::GridOverflow { columns, rows } => write!(
                formatter,
                "image grid tile count overflows this platform: {columns}x{rows}"
            ),
            Self::TileCountMismatch {
                columns,
                rows,
                expected,
                actual,
            } => write!(
                formatter,
                "image grid {columns}x{rows} requires {expected} tiles, found {actual}"
            ),
            Self::ColumnWidthMismatch {
                tile_index,
                column,
                expected,
                actual,
            } => write!(
                formatter,
                "tile {tile_index} in column {column} has width {actual}, expected {expected}"
            ),
            Self::RowHeightMismatch {
                tile_index,
                row,
                expected,
                actual,
            } => write!(
                formatter,
                "tile {tile_index} in row {row} has height {actual}, expected {expected}"
            ),
            Self::OutputDimensionMismatch { expected, actual } => write!(
                formatter,
                "assembled image dimensions are {}x{}, expected {}x{}",
                actual.0, actual.1, expected.0, expected.1
            ),
            Self::DimensionOverflow {
                output_width,
                output_height,
            } => write!(
                formatter,
                "assembled image dimensions overflow this platform: {output_width}x{output_height}"
            ),
            Self::OutputTooLarge { required, maximum } => write!(
                formatter,
                "assembled image requires {required} bytes, exceeding the {maximum}-byte limit"
            ),
        }
    }
}

impl Error for ImageAssemblyError {}

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

fn checked_dimension_sum(values: &[u32]) -> Option<u32> {
    values
        .iter()
        .try_fold(0_u32, |total, value| total.checked_add(*value))
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

    fn solid_tile(width: u32, height: u32, rgba: [u8; 4]) -> RgbaImage {
        let mut pixels = Vec::new();
        for _ in 0..width * height {
            pixels.extend_from_slice(&rgba);
        }
        RgbaImage {
            width,
            height,
            pixels,
        }
    }

    #[test]
    fn assembles_row_major_tiles_with_smaller_right_and_bottom_edges() {
        let tiles = [
            solid_tile(2, 1, [255, 0, 0, 255]),
            solid_tile(1, 1, [0, 255, 0, 255]),
            solid_tile(2, 2, [0, 0, 255, 255]),
            solid_tile(1, 2, [255, 255, 0, 255]),
        ];

        let image = RgbaImage::assemble_grid(&tiles, 2, 2, 3, 3, 36).expect("assemble grid");

        assert_eq!((image.width(), image.height()), (3, 3));
        assert_eq!(
            image.pixels(),
            &[
                255, 0, 0, 255, 255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 0, 0, 255, 255,
                255, 255, 0, 255, 0, 0, 255, 255, 0, 0, 255, 255, 255, 255, 0, 255,
            ]
        );
    }

    #[test]
    fn rejects_a_tile_whose_width_disagrees_with_its_column() {
        let tiles = [
            solid_tile(2, 1, [0; 4]),
            solid_tile(1, 1, [0; 4]),
            solid_tile(3, 1, [0; 4]),
            solid_tile(1, 1, [0; 4]),
        ];

        let error = RgbaImage::assemble_grid(&tiles, 2, 2, 3, 2, 24).unwrap_err();

        assert_eq!(
            error,
            ImageAssemblyError::ColumnWidthMismatch {
                tile_index: 2,
                column: 0,
                expected: 2,
                actual: 3,
            }
        );
    }

    #[test]
    fn rejects_a_tile_whose_height_disagrees_with_its_row() {
        let tiles = [solid_tile(1, 1, [0; 4]), solid_tile(1, 2, [0; 4])];

        let error = RgbaImage::assemble_grid(&tiles, 2, 1, 2, 1, 8).unwrap_err();

        assert_eq!(
            error,
            ImageAssemblyError::RowHeightMismatch {
                tile_index: 1,
                row: 0,
                expected: 1,
                actual: 2,
            }
        );
    }

    #[test]
    fn rejects_dimensions_that_disagree_with_the_tile_grid() {
        let tiles = [solid_tile(2, 2, [0; 4])];

        let error = RgbaImage::assemble_grid(&tiles, 1, 1, 3, 2, 24).unwrap_err();

        assert_eq!(
            error,
            ImageAssemblyError::OutputDimensionMismatch {
                expected: (3, 2),
                actual: (2, 2),
            }
        );
    }

    #[test]
    fn rejects_a_grid_with_missing_tiles() {
        let tiles = [solid_tile(1, 1, [0; 4])];

        let error = RgbaImage::assemble_grid(&tiles, 2, 1, 2, 1, 8).unwrap_err();

        assert_eq!(
            error,
            ImageAssemblyError::TileCountMismatch {
                columns: 2,
                rows: 1,
                expected: 2,
                actual: 1,
            }
        );
    }

    #[test]
    fn rejects_an_output_above_the_memory_limit() {
        let tiles = [solid_tile(2, 2, [0; 4])];

        let error = RgbaImage::assemble_grid(&tiles, 1, 1, 2, 2, 15).unwrap_err();

        assert_eq!(
            error,
            ImageAssemblyError::OutputTooLarge {
                required: 16,
                maximum: 15,
            }
        );
    }
}
