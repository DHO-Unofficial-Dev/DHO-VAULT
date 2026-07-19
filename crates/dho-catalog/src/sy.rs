// SPDX-License-Identifier: MPL-2.0

use crate::{CategoryPath, CategorySource, RecordRule, RuleScope};

const EVENT_ILLUSTRATION: CategoryPath = CategoryPath::new(&["이벤트", "삽화"]);

pub(crate) const RECORD_RULES: &[RecordRule] = &[RecordRule::verified(
    RuleScope::BlockRange { start: 0, end: 81 },
    EVENT_ILLUSTRATION,
    CategorySource::Custom,
)];
