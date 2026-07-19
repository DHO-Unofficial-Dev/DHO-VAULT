// SPDX-License-Identifier: MPL-2.0

use crate::{CategoryPath, CategorySource, RecordRule, RuleScope};

const COUNTRY_SELECTION_MAP: CategoryPath = CategoryPath::new(&["지도", "국가 선택 지도"]);

pub(crate) const RECORD_RULES: &[RecordRule] = &[RecordRule::verified(
    RuleScope::IdRange { start: 0, end: 5 },
    COUNTRY_SELECTION_MAP,
    CategorySource::Custom,
)];
