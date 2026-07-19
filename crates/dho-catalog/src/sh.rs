// SPDX-License-Identifier: MPL-2.0

use crate::{CategoryPath, CategorySource, RecordRule, RuleScope};

const CONSTELLATION_LINE_ART: CategoryPath =
    CategoryPath::new(&["UI 이미지", "별자리 조사", "별자리 선화 (256×256)"]);

pub(crate) const RECORD_RULES: &[RecordRule] = &[RecordRule::verified(
    RuleScope::BlockRange { start: 0, end: 87 },
    CONSTELLATION_LINE_ART,
    CategorySource::Custom,
)];
