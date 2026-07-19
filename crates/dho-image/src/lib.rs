// SPDX-License-Identifier: MPL-2.0

//! Pixel conversion and encoding for decoded DHO image resources.

use image::codecs::png::PngEncoder;
use image::imageops::FilterType;
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

    /// Expands exactly one grayscale byte per declared image position into opaque RGBA pixels.
    pub fn from_gray8(width: u32, height: u32, bytes: &[u8]) -> Result<Self, PixelDecodeError> {
        let expected_len = pixel_count(width, height)?;
        if bytes.len() != expected_len {
            return Err(PixelDecodeError::ByteLengthMismatch {
                width,
                height,
                expected_len,
                actual_len: bytes.len(),
            });
        }

        let rgba_len = pixel_byte_len(width, height)?;
        let mut pixels = Vec::with_capacity(rgba_len);
        for gray in bytes {
            pixels.extend_from_slice(&[*gray, *gray, *gray, 0xff]);
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

    /// Alpha-composites an equally sized RGBA layer over this image.
    pub fn overlay(&mut self, layer: &Self) -> Result<(), ImageAssemblyError> {
        if (self.width, self.height) != (layer.width, layer.height) {
            return Err(ImageAssemblyError::LayerDimensionMismatch {
                base: (self.width, self.height),
                layer: (layer.width, layer.height),
            });
        }

        for (base, layer) in self
            .pixels
            .chunks_exact_mut(4)
            .zip(layer.pixels.chunks_exact(4))
        {
            let source_alpha = u32::from(layer[3]);
            let base_alpha = u32::from(base[3]);
            let inverse_source_alpha = 255 - source_alpha;
            let output_alpha = source_alpha + (base_alpha * inverse_source_alpha + 127) / 255;
            if output_alpha == 0 {
                base.copy_from_slice(&[0, 0, 0, 0]);
                continue;
            }

            for channel in 0..3 {
                let premultiplied = u32::from(layer[channel]) * source_alpha * 255
                    + u32::from(base[channel]) * base_alpha * inverse_source_alpha;
                let divisor = output_alpha * 255;
                base[channel] = ((premultiplied + divisor / 2) / divisor) as u8;
            }
            base[3] = output_alpha as u8;
        }
        Ok(())
    }

    /// Shrinks an image to fit within the requested bounds without enlarging it.
    pub fn thumbnail(
        &self,
        max_width: u32,
        max_height: u32,
        max_output_size: usize,
    ) -> Result<Self, ThumbnailError> {
        if max_width == 0 || max_height == 0 {
            return Err(ThumbnailError::EmptyBounds {
                max_width,
                max_height,
            });
        }

        let (width, height) = thumbnail_dimensions(self.width, self.height, max_width, max_height)
            .ok_or(ThumbnailError::DimensionOverflow {
                width: self.width,
                height: self.height,
                max_width,
                max_height,
            })?;
        let required =
            pixel_byte_len(width, height).map_err(|_| ThumbnailError::DimensionOverflow {
                width: self.width,
                height: self.height,
                max_width,
                max_height,
            })?;
        if required > max_output_size {
            return Err(ThumbnailError::OutputTooLarge {
                required,
                maximum: max_output_size,
            });
        }
        if (width, height) == (self.width, self.height) {
            return Ok(self.clone());
        }

        let source = image::RgbaImage::from_raw(self.width, self.height, self.pixels.clone())
            .expect("validated RGBA pixels match the declared dimensions");
        let resized = image::imageops::resize(&source, width, height, FilterType::Triangle);
        Ok(Self {
            width,
            height,
            pixels: resized.into_raw(),
        })
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

fn thumbnail_dimensions(
    width: u32,
    height: u32,
    max_width: u32,
    max_height: u32,
) -> Option<(u32, u32)> {
    if width == 0 || height == 0 {
        return None;
    }
    if width <= max_width && height <= max_height {
        return Some((width, height));
    }

    if u64::from(width) * u64::from(max_height) > u64::from(height) * u64::from(max_width) {
        let numerator = u64::from(height) * u64::from(max_width);
        let scaled_height = ((numerator + u64::from(width) / 2) / u64::from(width)).max(1);
        Some((max_width, u32::try_from(scaled_height).ok()?))
    } else {
        let numerator = u64::from(width) * u64::from(max_height);
        let scaled_width = ((numerator + u64::from(height) / 2) / u64::from(height)).max(1);
        Some((u32::try_from(scaled_width).ok()?, max_height))
    }
}

/// Invalid bounds or resource limits while creating a display thumbnail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThumbnailError {
    EmptyBounds {
        max_width: u32,
        max_height: u32,
    },
    DimensionOverflow {
        width: u32,
        height: u32,
        max_width: u32,
        max_height: u32,
    },
    OutputTooLarge {
        required: usize,
        maximum: usize,
    },
}

impl fmt::Display for ThumbnailError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyBounds {
                max_width,
                max_height,
            } => write!(
                formatter,
                "thumbnail bounds must be non-zero: {max_width}x{max_height}"
            ),
            Self::DimensionOverflow {
                width,
                height,
                max_width,
                max_height,
            } => write!(
                formatter,
                "thumbnail dimensions overflow: source={width}x{height}, bounds={max_width}x{max_height}"
            ),
            Self::OutputTooLarge { required, maximum } => write!(
                formatter,
                "thumbnail requires {required} decoded bytes, exceeding the limit of {maximum}"
            ),
        }
    }
}

impl Error for ThumbnailError {}

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
                "decoded image byte length overflows this platform: width={width}, height={height}"
            ),
            Self::ByteLengthMismatch {
                width,
                height,
                expected_len,
                actual_len,
            } => write!(
                formatter,
                "decoded image byte length mismatch for {width}x{height}: expected {expected_len}, found {actual_len}"
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
    LayerDimensionMismatch {
        base: (u32, u32),
        layer: (u32, u32),
    },
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
            Self::LayerDimensionMismatch { base, layer } => write!(
                formatter,
                "overlay dimensions are {}x{}, expected {}x{}",
                layer.0, layer.1, base.0, base.1
            ),
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
    pixel_count(width, height)?
        .checked_mul(4)
        .ok_or(PixelDecodeError::DimensionOverflow { width, height })
}

fn pixel_count(width: u32, height: u32) -> Result<usize, PixelDecodeError> {
    usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .ok_or(PixelDecodeError::DimensionOverflow { width, height })
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
    fn expands_grayscale_pixels_to_opaque_rgba() {
        let image = RgbaImage::from_gray8(2, 1, &[0x11, 0xcc]).expect("valid grayscale pixels");

        assert_eq!(
            image.pixels(),
            &[0x11, 0x11, 0x11, 0xff, 0xcc, 0xcc, 0xcc, 0xff]
        );
    }

    #[test]
    fn alpha_composites_an_equally_sized_layer() {
        let mut base =
            RgbaImage::from_bgra(2, 1, &[30, 20, 10, 255, 0, 0, 0, 0]).expect("base pixels");
        let layer =
            RgbaImage::from_bgra(2, 1, &[110, 100, 90, 128, 7, 6, 5, 255]).expect("layer pixels");

        base.overlay(&layer).expect("overlay images");

        assert_eq!(base.pixels(), &[50, 60, 70, 255, 5, 6, 7, 255]);
    }

    #[test]
    fn rejects_an_overlay_with_different_dimensions() {
        let mut base = RgbaImage::from_bgra(1, 1, &[0; 4]).expect("base pixels");
        let layer = RgbaImage::from_bgra(2, 1, &[0; 8]).expect("layer pixels");

        assert_eq!(
            base.overlay(&layer),
            Err(ImageAssemblyError::LayerDimensionMismatch {
                base: (1, 1),
                layer: (2, 1),
            })
        );
    }

    #[test]
    fn rejects_an_incorrect_grayscale_byte_length() {
        let error = RgbaImage::from_gray8(2, 2, &[0; 3]).unwrap_err();

        assert_eq!(
            error,
            PixelDecodeError::ByteLengthMismatch {
                width: 2,
                height: 2,
                expected_len: 4,
                actual_len: 3,
            }
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
    fn shrinks_a_thumbnail_without_changing_its_aspect_ratio() {
        let image = solid_tile(8, 4, [10, 20, 30, 255]);

        let thumbnail = image.thumbnail(3, 3, 24).expect("create thumbnail");

        assert_eq!((thumbnail.width(), thumbnail.height()), (3, 2));
        assert_eq!(thumbnail.pixels().len(), 24);
    }

    #[test]
    fn does_not_enlarge_an_image_that_already_fits() {
        let image = solid_tile(2, 3, [10, 20, 30, 255]);

        let thumbnail = image.thumbnail(160, 160, 24).expect("keep image size");

        assert_eq!(thumbnail, image);
    }

    #[test]
    fn rejects_empty_thumbnail_bounds() {
        let image = solid_tile(2, 3, [10, 20, 30, 255]);

        let error = image.thumbnail(0, 160, 24).unwrap_err();

        assert_eq!(
            error,
            ThumbnailError::EmptyBounds {
                max_width: 0,
                max_height: 160,
            }
        );
    }

    #[test]
    fn enforces_the_thumbnail_output_limit() {
        let image = solid_tile(4, 4, [10, 20, 30, 255]);

        let error = image.thumbnail(4, 4, 63).unwrap_err();

        assert_eq!(
            error,
            ThumbnailError::OutputTooLarge {
                required: 64,
                maximum: 63,
            }
        );
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
