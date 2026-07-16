// SPDX-License-Identifier: MPL-2.0

use crate::{CategoryPath, CategorySource, RecordRule, ReservationRule, RuleScope};

const EQUIPMENT_BODY: CategoryPath = CategoryPath::new(&["장비", "방어구", "몸"]);
const EQUIPMENT_HEAD: CategoryPath = CategoryPath::new(&["장비", "방어구", "머리"]);
const EQUIPMENT_LEGS: CategoryPath = CategoryPath::new(&["장비", "방어구", "다리"]);
const EQUIPMENT_ARMS: CategoryPath = CategoryPath::new(&["장비", "방어구", "팔"]);
const EQUIPMENT_WEAPON: CategoryPath = CategoryPath::new(&["장비", "무기"]);
const EQUIPMENT_TOOL: CategoryPath = CategoryPath::new(&["장비", "도구"]);
const SHIP_AUXILIARY_SAIL: CategoryPath = CategoryPath::new(&["선박", "선박 장비", "보조돛"]);
const SHIP_CANNON: CategoryPath = CategoryPath::new(&["선박", "선박 장비", "대포"]);
const SHIP_ADDITIONAL_ARMOR: CategoryPath = CategoryPath::new(&["선박", "선박 장비", "추가장갑"]);
const SHIP_SPECIAL_EQUIPMENT: CategoryPath = CategoryPath::new(&["선박", "선박 장비", "특수장비"]);
const SHIP_FIGUREHEAD: CategoryPath = CategoryPath::new(&["선박", "선박 장비", "선수상"]);
const SHIP_EMBLEM: CategoryPath = CategoryPath::new(&["선박", "선박 장비", "문장"]);
const TEMPORARY_SPECIAL_ICONS: CategoryPath = CategoryPath::new(&["미분류", "SB 특수 아이콘"]);
const SHIP_SUPPLIES: CategoryPath = CategoryPath::new(&["선박", "선박 물자"]);
const ITEM_CONSUMABLE: CategoryPath = CategoryPath::new(&["아이템", "소비품"]);
const ITEM_RECOMMENDATION: CategoryPath = CategoryPath::new(&["아이템", "추천장"]);
const ITEM_RECIPE: CategoryPath = CategoryPath::new(&["아이템", "레시피"]);
const ITEM_TREASURE_MAP: CategoryPath = CategoryPath::new(&["아이템", "보물지도"]);
const ITEM_ACQUISITION: CategoryPath = CategoryPath::new(&["아이템", "아이템 획득"]);
const SHIP_MATERIAL: CategoryPath = CategoryPath::new(&["선박", "선박 재료"]);
const ITEM_DECORATION: CategoryPath = CategoryPath::new(&["아이템", "장식품"]);
const ITEM_PET_CONSUMABLE: CategoryPath = CategoryPath::new(&["아이템", "소비품", "애완동물"]);
const SHIP_DECO: CategoryPath = CategoryPath::new(&["선박", "선박 데코"]);
const SHIP_CREW_EQUIPMENT: CategoryPath = CategoryPath::new(&["선박", "선원 장비"]);

const fn verified_range(start: u32, end: u32, category: CategoryPath) -> RecordRule {
    RecordRule::verified(
        RuleScope::IdRange { start, end },
        category,
        CategorySource::InGame,
    )
}

const fn custom_range(start: u32, end: u32, category: CategoryPath) -> RecordRule {
    RecordRule::verified(
        RuleScope::IdRange { start, end },
        category,
        CategorySource::Custom,
    )
}

const fn custom_exact(icon_id: u32, category: CategoryPath) -> RecordRule {
    RecordRule::verified(
        RuleScope::ExactId(icon_id),
        category,
        CategorySource::Custom,
    )
}

pub(crate) const RECORD_RULES: &[RecordRule] = &[
    verified_range(0, 99_999, EQUIPMENT_BODY),
    verified_range(100_000, 199_999, EQUIPMENT_HEAD),
    verified_range(200_000, 299_999, EQUIPMENT_LEGS),
    verified_range(300_000, 399_999, EQUIPMENT_ARMS),
    verified_range(400_000, 499_999, EQUIPMENT_WEAPON),
    verified_range(500_000, 599_999, EQUIPMENT_TOOL),
    verified_range(600_000, 699_999, SHIP_AUXILIARY_SAIL),
    verified_range(700_000, 799_999, SHIP_CANNON),
    verified_range(800_000, 899_999, SHIP_ADDITIONAL_ARMOR),
    verified_range(900_000, 999_999, SHIP_SPECIAL_EQUIPMENT),
    verified_range(1_000_000, 1_099_999, SHIP_FIGUREHEAD),
    verified_range(1_100_000, 1_199_999, SHIP_EMBLEM),
    RecordRule::explicit_unknown(RuleScope::ExactId(1_200_001)),
    RecordRule::explicit_unknown(RuleScope::ExactId(1_200_101)),
    RecordRule::temporary(RuleScope::ExactId(1_300_001), TEMPORARY_SPECIAL_ICONS),
    RecordRule::temporary(RuleScope::ExactId(1_300_100), TEMPORARY_SPECIAL_ICONS),
    RecordRule::temporary(
        RuleScope::IdRange {
            start: 1_390_001,
            end: 1_390_015,
        },
        TEMPORARY_SPECIAL_ICONS,
    ),
    custom_exact(1_400_100, SHIP_SUPPLIES),
    custom_exact(1_400_200, SHIP_SUPPLIES),
    custom_exact(1_400_300, SHIP_SUPPLIES),
    custom_exact(1_400_400, SHIP_SUPPLIES),
    verified_range(1_500_000, 1_599_999, ITEM_CONSUMABLE),
    verified_range(1_700_000, 1_799_999, ITEM_RECOMMENDATION),
    verified_range(1_800_000, 1_899_999, ITEM_RECIPE),
    verified_range(1_900_000, 1_999_999, ITEM_TREASURE_MAP),
    custom_range(2_000_000, 2_199_999, ITEM_ACQUISITION),
    verified_range(2_200_000, 2_299_999, SHIP_MATERIAL),
    verified_range(2_300_000, 2_399_999, ITEM_DECORATION),
    verified_range(2_400_000, 2_499_999, ITEM_PET_CONSUMABLE),
    verified_range(2_500_000, 2_599_999, SHIP_DECO),
    verified_range(2_600_001, 2_602_160, SHIP_CREW_EQUIPMENT),
    RecordRule::explicit_unknown(RuleScope::ExactId(2_602_161)),
];

const fn reservation(start: u32, end: u32, category: CategoryPath) -> ReservationRule {
    ReservationRule::new(start, end, category, CategorySource::InGame)
}

pub(crate) const RESERVATION_RULES: &[ReservationRule] = &[
    reservation(0, 99_999, EQUIPMENT_BODY),
    reservation(100_000, 199_999, EQUIPMENT_HEAD),
    reservation(200_000, 299_999, EQUIPMENT_LEGS),
    reservation(300_000, 399_999, EQUIPMENT_ARMS),
    reservation(400_000, 499_999, EQUIPMENT_WEAPON),
    reservation(500_000, 599_999, EQUIPMENT_TOOL),
    reservation(600_000, 699_999, SHIP_AUXILIARY_SAIL),
    reservation(700_000, 799_999, SHIP_CANNON),
    reservation(800_000, 899_999, SHIP_ADDITIONAL_ARMOR),
    reservation(900_000, 999_999, SHIP_SPECIAL_EQUIPMENT),
    reservation(1_000_000, 1_099_999, SHIP_FIGUREHEAD),
    reservation(1_100_000, 1_199_999, SHIP_EMBLEM),
    reservation(1_500_000, 1_599_999, ITEM_CONSUMABLE),
    reservation(1_700_000, 1_799_999, ITEM_RECOMMENDATION),
    reservation(1_800_000, 1_899_999, ITEM_RECIPE),
    reservation(1_900_000, 1_999_999, ITEM_TREASURE_MAP),
    reservation(2_200_000, 2_299_999, SHIP_MATERIAL),
    reservation(2_300_000, 2_399_999, ITEM_DECORATION),
    reservation(2_400_000, 2_499_999, ITEM_PET_CONSUMABLE),
    reservation(2_500_000, 2_599_999, SHIP_DECO),
    reservation(2_600_000, 2_699_999, SHIP_CREW_EQUIPMENT),
];
