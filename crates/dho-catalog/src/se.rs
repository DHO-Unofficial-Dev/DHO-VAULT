// SPDX-License-Identifier: MPL-2.0

use crate::{CategoryPath, CategorySource, RecordRule, RuleScope};

const UI_TEXT_LABEL: CategoryPath = CategoryPath::new(&["UI 이미지", "텍스트 라벨"]);

pub(crate) const RECORD_RULES: &[RecordRule] = &[RecordRule::verified(
    RuleScope::Group(0),
    UI_TEXT_LABEL,
    CategorySource::Custom,
)];
