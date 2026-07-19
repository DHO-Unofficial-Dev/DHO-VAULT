// SPDX-License-Identifier: MPL-2.0

use crate::{CategoryPath, CategorySource, RecordRule, RuleScope};

const CLIENT_LOADING_SPLASH: CategoryPath =
    CategoryPath::new(&["클라이언트", "로딩·스플래시 이미지"]);

pub(crate) const RECORD_RULES: &[RecordRule] = &[RecordRule::verified(
    RuleScope::IdRange {
        start: 0,
        end: u32::MAX,
    },
    CLIENT_LOADING_SPLASH,
    CategorySource::Custom,
)];
