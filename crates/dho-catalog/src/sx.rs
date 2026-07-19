// SPDX-License-Identifier: MPL-2.0

use crate::{CategoryPath, CategorySource, RecordRule, RuleScope};

const EVENT_PORTRAIT: CategoryPath = CategoryPath::new(&["이벤트", "포트레잇"]);

pub(crate) const RECORD_RULES: &[RecordRule] = &[RecordRule::verified(
    RuleScope::BlockRange { start: 0, end: 219 },
    EVENT_PORTRAIT,
    CategorySource::Custom,
)];
