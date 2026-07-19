// SPDX-License-Identifier: MPL-2.0

use crate::{CategoryPath, CategorySource, RecordRule, RuleScope};

const UI_BUTTON: CategoryPath = CategoryPath::new(&["UI 이미지", "버튼"]);

pub(crate) const RECORD_RULES: &[RecordRule] = &[RecordRule::verified(
    RuleScope::BlockRange {
        start: 0,
        end: 1_145,
    },
    UI_BUTTON,
    CategorySource::Custom,
)];
