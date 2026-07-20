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

    const fn verified_is(
        start_block: u32,
        end_block: u32,
        output_width: u32,
        output_height: u32,
    ) -> Self {
        Self {
            archive: "is",
            start_block,
            end_block,
            tiles_per_image: 12,
            columns: 4,
            rows: 3,
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

/// One human-verified logical image composed from matching tiles in two raw files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayeredAssemblyRule {
    pub archive: &'static str,
    pub base_file_number: u32,
    pub overlay_file_number: u32,
    pub first_file_block_index: u32,
    pub tile_count: u32,
    pub columns: u32,
    pub rows: u32,
    pub output_width: u32,
    pub output_height: u32,
    pub canonical_block: u32,
    pub last_block: u32,
    pub status: VerificationStatus,
}

impl LayeredAssemblyRule {
    /// Whether a physical raw block contributes to this completed image.
    pub const fn contains_source(self, file_number: u32, file_block_index: u32) -> bool {
        let source_file =
            file_number == self.base_file_number || file_number == self.overlay_file_number;
        source_file
            && self.first_file_block_index <= file_block_index
            && file_block_index < self.first_file_block_index.saturating_add(self.tile_count)
    }
}

pub(crate) const RULES: &[AssemblyRule] = &[
    AssemblyRule::verified_sd(4_027, 6_267, 3, 3, 378, 294),
    AssemblyRule::verified_sd(6_277, 7_932, 3, 3, 384, 384),
    AssemblyRule::verified_sd(7_933, 8_718, 3, 2, 320, 220),
    AssemblyRule::verified_sd(8_842, 8_856, 5, 3, 640, 320),
    AssemblyRule::verified_sd(9_291, 9_978, 2, 2, 192, 192),
    AssemblyRule::verified_sd(10_156, 10_175, 2, 2, 155, 256),
    AssemblyRule::verified_sd(10_203, 10_242, 2, 2, 248, 156),
    AssemblyRule::verified_sd(10_368, 10_395, 7, 4, 782, 404),
    AssemblyRule::verified_sd(10_396, 10_399, 2, 2, 256, 256),
    AssemblyRule::verified_sd(10_400, 10_405, 3, 2, 294, 166),
    AssemblyRule::verified_sd(10_406, 10_409, 2, 2, 166, 166),
    AssemblyRule::verified_sd(10_419, 10_438, 2, 1, 256, 128),
    AssemblyRule::verified_sd(10_439, 10_470, 8, 4, 1_024, 512),
    AssemblyRule::verified_sd(10_617, 10_800, 2, 2, 192, 192),
    AssemblyRule::verified_is(0, 11, 1_024, 768),
    AssemblyRule::verified_is(12, 23, 1_024, 768),
    AssemblyRule::verified_is(24, 35, 864, 664),
    AssemblyRule::verified_is(36, 47, 864, 664),
    AssemblyRule::verified_is(48, 59, 864, 664),
    AssemblyRule::verified_is(60, 71, 864, 664),
    AssemblyRule::verified_is(72, 83, 864, 664),
    AssemblyRule::verified_is(84, 95, 864, 664),
    AssemblyRule::verified_is(96, 107, 800, 600),
];

pub(crate) const LAYERED_RULES: &[LayeredAssemblyRule] = &[LayeredAssemblyRule {
    archive: "kp",
    base_file_number: 0,
    overlay_file_number: 10,
    first_file_block_index: 0,
    tile_count: 2_048,
    columns: 64,
    rows: 32,
    output_width: 3_072,
    output_height: 1_536,
    canonical_block: 0,
    last_block: 4_095,
    status: VerificationStatus::HumanVerified,
}];

pub(crate) fn find_plan(archive: &str, block_index: u32) -> Option<AssemblyPlan> {
    find_plan_with_status(archive, block_index, VerificationStatus::HumanVerified)
}

pub(crate) fn find_candidate_plan(archive: &str, block_index: u32) -> Option<AssemblyPlan> {
    find_plan_with_status(archive, block_index, VerificationStatus::Candidate)
}

pub(crate) fn find_layered_rule(archive: &str) -> Option<LayeredAssemblyRule> {
    LAYERED_RULES.iter().copied().find(|rule| {
        rule.archive.eq_ignore_ascii_case(archive)
            && rule.status == VerificationStatus::HumanVerified
    })
}

fn find_plan_with_status(
    archive: &str,
    block_index: u32,
    status: VerificationStatus,
) -> Option<AssemblyPlan> {
    RULES.iter().copied().find_map(|rule| {
        (rule.archive.eq_ignore_ascii_case(archive) && rule.status == status)
            .then(|| rule.plan_for(block_index))
            .flatten()
    })
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

    #[test]
    fn resolves_all_verified_is_images_without_leaving_candidates() {
        let expected = [
            (0, 11, 1_024, 768),
            (12, 23, 1_024, 768),
            (24, 35, 864, 664),
            (36, 47, 864, 664),
            (48, 59, 864, 664),
            (60, 71, 864, 664),
            (72, 83, 864, 664),
            (84, 95, 864, 664),
            (96, 107, 800, 600),
        ];

        for (first_block, last_block, width, height) in expected {
            let plan = find_plan("IS", first_block).expect("verified IS plan");
            assert_eq!(plan.first_block, first_block);
            assert_eq!(plan.last_block, last_block);
            assert_eq!(plan.rule.output_width, width);
            assert_eq!(plan.rule.output_height, height);
            assert_eq!(plan.rule.status, VerificationStatus::HumanVerified);
            assert_eq!(find_candidate_plan("is", first_block), None);
        }
    }

    #[test]
    fn resolves_the_verified_kp_layered_world_map() {
        let rule = find_layered_rule("KP").expect("verified KP layered rule");

        assert_eq!((rule.columns, rule.rows), (64, 32));
        assert_eq!(rule.tile_count, rule.columns * rule.rows);
        assert_eq!((rule.output_width, rule.output_height), (3_072, 1_536));
        assert!(rule.contains_source(0, 0));
        assert!(rule.contains_source(10, 2_047));
        assert!(!rule.contains_source(100_000, 0));
        assert!(!rule.contains_source(10, 2_048));
    }
}
