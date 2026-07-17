// SPDX-License-Identifier: MPL-2.0

use serde::Serialize;

use crate::VerificationStatus;

/// The order used to place consecutive physical image blocks in a grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TileOrder {
    RowMajor,
}

/// One human-verified rule shared by every completed image in a block range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssemblyRule {
    pub archive: &'static str,
    pub start_block: u32,
    pub end_block: u32,
    pub tiles_per_image: u32,
    pub columns: u32,
    pub rows: u32,
    pub output_width: u32,
    pub output_height: u32,
    pub tile_order: TileOrder,
    pub status: VerificationStatus,
}

impl AssemblyRule {
    const fn verified_sd(
        start_block: u32,
        end_block: u32,
        columns: u32,
        rows: u32,
        output_width: u32,
        output_height: u32,
    ) -> Self {
        Self {
            archive: "sd",
            start_block,
            end_block,
            tiles_per_image: columns * rows,
            columns,
            rows,
            output_width,
            output_height,
            tile_order: TileOrder::RowMajor,
            status: VerificationStatus::HumanVerified,
        }
    }

    const fn contains(self, block_index: u32) -> bool {
        self.start_block <= block_index && block_index <= self.end_block
    }

    /// Number of completed images represented by this rule.
    pub const fn image_count(self) -> u32 {
        (self.end_block - self.start_block + 1) / self.tiles_per_image
    }

    fn plan_for(self, block_index: u32) -> Option<AssemblyPlan> {
        if !self.contains(block_index) {
            return None;
        }

        let offset = block_index - self.start_block;
        let image_index = offset / self.tiles_per_image;
        let tile_index = offset % self.tiles_per_image;
        let first_block = self.start_block + image_index * self.tiles_per_image;

        Some(AssemblyPlan {
            rule: self,
            image_index,
            first_block,
            last_block: first_block + self.tiles_per_image - 1,
            tile_index,
            row: tile_index / self.columns,
            column: tile_index % self.columns,
        })
    }
}

/// The exact completed image and tile position containing one physical block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssemblyPlan {
    pub rule: AssemblyRule,
    pub image_index: u32,
    pub first_block: u32,
    pub last_block: u32,
    pub tile_index: u32,
    pub row: u32,
    pub column: u32,
}

pub(crate) const RULES: &[AssemblyRule] = &[
    AssemblyRule::verified_sd(10_156, 10_175, 2, 2, 155, 256),
    AssemblyRule::verified_sd(10_203, 10_242, 2, 2, 248, 156),
    AssemblyRule::verified_sd(10_368, 10_395, 7, 4, 782, 404),
    AssemblyRule::verified_sd(10_396, 10_399, 2, 2, 256, 256),
    AssemblyRule::verified_sd(10_400, 10_405, 3, 2, 294, 166),
    AssemblyRule::verified_sd(10_406, 10_409, 2, 2, 166, 166),
    AssemblyRule::verified_sd(10_439, 10_470, 8, 4, 1_024, 512),
    AssemblyRule::verified_sd(10_617, 10_800, 2, 2, 192, 192),
];

pub(crate) fn find_plan(archive: &str, block_index: u32) -> Option<AssemblyPlan> {
    archive
        .eq_ignore_ascii_case("sd")
        .then(|| {
            RULES
                .iter()
                .copied()
                .find_map(|rule| rule.plan_for(block_index))
        })
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verified_rules_have_complete_non_overlapping_grids() {
        for (index, rule) in RULES.iter().copied().enumerate() {
            assert_eq!(rule.tiles_per_image, rule.columns * rule.rows);
            assert_eq!(
                (rule.end_block - rule.start_block + 1) % rule.tiles_per_image,
                0
            );
            assert!(rule.output_width > 0);
            assert!(rule.output_height > 0);

            for next in RULES.iter().copied().skip(index + 1) {
                assert!(rule.end_block < next.start_block || next.end_block < rule.start_block);
            }
        }
    }
}
