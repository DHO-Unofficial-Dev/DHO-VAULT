// SPDX-License-Identifier: MPL-2.0

use crate::{CategoryPath, CategorySource, RecordRule, RuleScope};

const TRADE_GOODS: CategoryPath = CategoryPath::new(&["교역품"]);
const SHIP: CategoryPath = CategoryPath::new(&["선박"]);
const SHIP_MATERIAL_TEXTURE: CategoryPath = CategoryPath::new(&["선박", "선박 재료", "재질"]);
const CITY_BUILDING: CategoryPath = CategoryPath::new(&["도시", "도시 내 건물"]);
const SHIP_SAIL_PATTERN: CategoryPath = CategoryPath::new(&["선박", "돛 무늬"]);
const UI_TAVERN_MENU: CategoryPath = CategoryPath::new(&["UI 아이콘", "주점 메뉴"]);
const UI_CARD: CategoryPath = CategoryPath::new(&["UI 아이콘", "카드"]);
const EVENT: CategoryPath = CategoryPath::new(&["이벤트"]);
const PERSON_AIDE_ICON: CategoryPath = CategoryPath::new(&["인물", "부관 아이콘"]);
const UI_TAROT: CategoryPath = CategoryPath::new(&["UI 아이콘", "타로"]);
const APARTAMENTO: CategoryPath = CategoryPath::new(&["아팔타멘토"]);
const DISCOVERY_SET_1_SMALL: CategoryPath =
    CategoryPath::new(&["발견물", "1", "리스트 이미지 (48×48)"]);
const APARTAMENTO_BUTLER: CategoryPath = CategoryPath::new(&["아팔타멘토", "집사"]);
const PRIVATE_FARM: CategoryPath = CategoryPath::new(&["개인농장"]);
const UI_PRODUCTION: CategoryPath = CategoryPath::new(&["UI 아이콘", "생산"]);
const UI_TECHNIQUE: CategoryPath = CategoryPath::new(&["UI 아이콘", "테크닉"]);
const COMPANY_COLONY: CategoryPath = CategoryPath::new(&["개척도시"]);
const EDUCATION_SYSTEMS: CategoryPath = CategoryPath::new(&["대학·학술협회·부관학교"]);
const PERSON_UNCLASSIFIED_PORTRAIT: CategoryPath = CategoryPath::new(&["인물", "미분류 초상화"]);
const MISCELLANEOUS: CategoryPath = CategoryPath::new(&["기타"]);
const DISCOVERY_SET_2_SMALL: CategoryPath =
    CategoryPath::new(&["발견물", "2", "리스트 이미지 (48×48)"]);
const PERSON_NPC_PORTRAIT: CategoryPath = CategoryPath::new(&["인물", "NPC 초상화"]);
const POTENTIAL: CategoryPath = CategoryPath::new(&["잠재능력"]);

const fn verified_block_range(
    start: u32,
    end: u32,
    category: CategoryPath,
    source: CategorySource,
) -> RecordRule {
    RecordRule::verified(RuleScope::BlockRange { start, end }, category, source)
}

pub(crate) const RECORD_RULES: &[RecordRule] = &[
    verified_block_range(0, 682, TRADE_GOODS, CategorySource::InGame),
    verified_block_range(683, 926, SHIP, CategorySource::InGame),
    verified_block_range(927, 997, SHIP_MATERIAL_TEXTURE, CategorySource::InGame),
    verified_block_range(998, 1_027, CITY_BUILDING, CategorySource::Custom),
    verified_block_range(1_028, 1_134, SHIP_SAIL_PATTERN, CategorySource::InGame),
    verified_block_range(1_135, 1_269, UI_TAVERN_MENU, CategorySource::Custom),
    verified_block_range(1_270, 1_322, UI_CARD, CategorySource::Custom),
    verified_block_range(1_323, 1_336, EVENT, CategorySource::Custom),
    verified_block_range(1_337, 1_472, PERSON_AIDE_ICON, CategorySource::Custom),
    verified_block_range(1_473, 1_494, UI_TAROT, CategorySource::Custom),
    verified_block_range(1_495, 1_506, APARTAMENTO, CategorySource::InGame),
    verified_block_range(1_507, 4_433, DISCOVERY_SET_1_SMALL, CategorySource::Custom),
    verified_block_range(4_434, 4_457, APARTAMENTO_BUTLER, CategorySource::InGame),
    // 4458..=4475 remains a candidate for Apartamento pets.
    verified_block_range(4_476, 4_489, PRIVATE_FARM, CategorySource::InGame),
    verified_block_range(4_490, 4_497, UI_PRODUCTION, CategorySource::Custom),
    verified_block_range(4_498, 4_531, UI_TECHNIQUE, CategorySource::Custom),
    verified_block_range(4_532, 4_655, COMPANY_COLONY, CategorySource::InGame),
    verified_block_range(4_656, 4_919, EDUCATION_SYSTEMS, CategorySource::Custom),
    RecordRule::explicit_unknown(RuleScope::ExactBlock(4_920)),
    verified_block_range(
        4_921,
        5_114,
        PERSON_UNCLASSIFIED_PORTRAIT,
        CategorySource::Custom,
    ),
    verified_block_range(5_115, 5_232, MISCELLANEOUS, CategorySource::Custom),
    verified_block_range(5_233, 5_265, DISCOVERY_SET_2_SMALL, CategorySource::Custom),
    verified_block_range(5_266, 5_310, PERSON_NPC_PORTRAIT, CategorySource::Custom),
    // 5311..=5326 remains a candidate for aide rescue portraits.
    verified_block_range(
        5_327,
        5_335,
        PERSON_UNCLASSIFIED_PORTRAIT,
        CategorySource::Custom,
    ),
    verified_block_range(5_336, 5_592, MISCELLANEOUS, CategorySource::Custom),
    verified_block_range(5_593, 5_620, POTENTIAL, CategorySource::InGame),
    verified_block_range(5_621, 5_940, MISCELLANEOUS, CategorySource::Custom),
];
