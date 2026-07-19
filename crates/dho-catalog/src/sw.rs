// SPDX-License-Identifier: MPL-2.0

use crate::{CategoryPath, CategorySource, RecordRule, RuleScope};

const PERSON_CHARACTER_FACE: CategoryPath = CategoryPath::new(&["인물", "캐릭터 얼굴"]);

pub(crate) const RECORD_RULES: &[RecordRule] = &[RecordRule::verified(
    RuleScope::BlockRange { start: 0, end: 132 },
    PERSON_CHARACTER_FACE,
    CategorySource::Custom,
)];
