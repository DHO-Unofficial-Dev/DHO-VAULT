// SPDX-License-Identifier: MPL-2.0

use crate::{CategoryPath, CategorySource, RecordRule, RuleScope};

const UI_BUTTON: CategoryPath = CategoryPath::new(&["UI 이미지", "버튼"]);
const UI_CIRCULAR_ICON: CategoryPath = CategoryPath::new(&["UI 아이콘", "원형 아이콘"]);

pub(crate) const RECORD_RULES: &[RecordRule] = &[
    RecordRule::verified(
        RuleScope::BlockRange {
            start: 0,
            end: 1_135,
        },
        UI_BUTTON,
        CategorySource::Custom,
    ),
    RecordRule::verified(
        RuleScope::BlockRange {
            start: 1_136,
            end: 1_410,
        },
        UI_CIRCULAR_ICON,
        CategorySource::Custom,
    ),
];
