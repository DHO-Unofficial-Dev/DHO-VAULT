// SPDX-License-Identifier: MPL-2.0

use crate::{CategoryPath, CategorySource, RecordRule, RuleScope};

const WORLD_MAP: CategoryPath = CategoryPath::new(&["지도", "세계지도 (3072×1536)"]);
const UNCLASSIFIED_OVERVIEW: CategoryPath = CategoryPath::new(&["지도", "미분류 오버뷰 (256×256)"]);

pub(crate) const RECORD_RULES: &[RecordRule] = &[
    RecordRule::verified(
        RuleScope::BlockRange {
            start: 0,
            end: 4_095,
        },
        WORLD_MAP,
        CategorySource::Custom,
    ),
    RecordRule::verified(
        RuleScope::BlockRange {
            start: 4_096,
            end: 4_100,
        },
        UNCLASSIFIED_OVERVIEW,
        CategorySource::Temporary,
    ),
];
