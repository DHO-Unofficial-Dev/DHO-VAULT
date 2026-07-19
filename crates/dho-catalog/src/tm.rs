// SPDX-License-Identifier: MPL-2.0

use crate::{CategoryPath, CategorySource, RecordRule, RuleScope};

const CITY_PORT_MINIMAP: CategoryPath =
    CategoryPath::new(&["지도", "도시·항구 미니맵 (180×139~141)"]);

pub(crate) const RECORD_RULES: &[RecordRule] = &[RecordRule::verified(
    RuleScope::BlockRange { start: 0, end: 248 },
    CITY_PORT_MINIMAP,
    CategorySource::Custom,
)];
