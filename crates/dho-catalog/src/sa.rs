// SPDX-License-Identifier: MPL-2.0

use crate::{CategoryPath, CategorySource, RecordRule, RuleScope};

const PERSON_AIDE_SKILL: CategoryPath = CategoryPath::new(&["인물", "부관 스킬"]);
const UI_EMOTION: CategoryPath = CategoryPath::new(&["UI 아이콘", "감정표현"]);
const SHIP_GRADE_BONUS: CategoryPath = CategoryPath::new(&["선박", "선박 그레이드 보너스"]);
const UI_ALCHEMY: CategoryPath = CategoryPath::new(&["UI 아이콘", "연금술"]);

const fn verified_block_range(
    start: u32,
    end: u32,
    category: CategoryPath,
    source: CategorySource,
) -> RecordRule {
    RecordRule::verified(RuleScope::BlockRange { start, end }, category, source)
}

pub(crate) const RECORD_RULES: &[RecordRule] = &[
    RecordRule::verified(
        RuleScope::Group(0),
        PERSON_AIDE_SKILL,
        CategorySource::InGame,
    ),
    verified_block_range(829, 829, PERSON_AIDE_SKILL, CategorySource::InGame),
    verified_block_range(830, 909, UI_EMOTION, CategorySource::Custom),
    verified_block_range(910, 910, PERSON_AIDE_SKILL, CategorySource::InGame),
    verified_block_range(911, 931, SHIP_GRADE_BONUS, CategorySource::InGame),
    verified_block_range(932, 938, PERSON_AIDE_SKILL, CategorySource::InGame),
    verified_block_range(939, 941, SHIP_GRADE_BONUS, CategorySource::InGame),
    verified_block_range(942, 943, UI_ALCHEMY, CategorySource::Custom),
];
