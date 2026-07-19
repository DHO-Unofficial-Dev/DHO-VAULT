// SPDX-License-Identifier: MPL-2.0

use crate::{CategoryPath, CategorySource, RecordRule, RuleScope};

const DISCOVERY_SET_1_LARGE: CategoryPath =
    CategoryPath::new(&["발견물", "1", "획득 이미지 (128×128)"]);
const BATTLE: CategoryPath = CategoryPath::new(&["전투"]);
const PORT_PERMIT_ACQUISITION: CategoryPath =
    CategoryPath::new(&["입항허가", "획득 이미지 (128×128)"]);
const HELP: CategoryPath = CategoryPath::new(&["도움말"]);
const HELP_SCREEN: CategoryPath = CategoryPath::new(&["도움말", "화면 이미지 (320×220)"]);
const CLIENT_SPLASH_UI: CategoryPath = CategoryPath::new(&["클라이언트", "스플래시 UI"]);
const WORLD_MAP: CategoryPath = CategoryPath::new(&["지도", "세계지도"]);
const FIELD_MAP: CategoryPath = CategoryPath::new(&["지도", "필드 지도"]);
const DISCOVERY_SET_2_LARGE: CategoryPath =
    CategoryPath::new(&["발견물", "2", "획득 이미지 (128×128)"]);
const DUNGEON_MAP: CategoryPath = CategoryPath::new(&["지도", "던전 지도"]);
const BLESSING: CategoryPath = CategoryPath::new(&["가호"]);
const EVENT: CategoryPath = CategoryPath::new(&["이벤트"]);
const PERSON_UNCLASSIFIED_PORTRAIT: CategoryPath = CategoryPath::new(&["인물", "미분류 초상화"]);
const CLIENT_PLACEHOLDER: CategoryPath = CategoryPath::new(&["클라이언트", "플레이스홀더"]);
const LEGEND_UNDISCOVERED: CategoryPath = CategoryPath::new(&["전승", "미발견 이미지 (128×128)"]);
const RESCUED_AIDE_PORTRAIT: CategoryPath = CategoryPath::new(&["인물", "구조 부관", "초상화"]);
const RESCUED_AIDE_IMAGE: CategoryPath = CategoryPath::new(&["인물", "구조 부관", "구조 이미지"]);
const PROPHECY_BOOK_COVER: CategoryPath = CategoryPath::new(&["UI 이미지", "예지의 서", "표지"]);
const LEGACY_THEME_UI: CategoryPath =
    CategoryPath::new(&["UI 이미지", "예지의 서", "유산의 장", "레거시 테마 UI"]);
const LEGACY_DETAIL_UI: CategoryPath =
    CategoryPath::new(&["UI 이미지", "예지의 서", "유산의 장", "레거시 상세 UI"]);
const CONSTELLATION_SKY_MAP: CategoryPath =
    CategoryPath::new(&["UI 이미지", "별자리 조사", "천구도"]);
const CONSTELLATION_LINES: CategoryPath =
    CategoryPath::new(&["UI 이미지", "별자리 조사", "별자리선 표시"]);
const CONSTELLATION_LINES_AND_ART: CategoryPath =
    CategoryPath::new(&["UI 이미지", "별자리 조사", "별자리선과 그림 표시"]);
const CONSTELLATION_IMAGE: CategoryPath =
    CategoryPath::new(&["UI 이미지", "별자리 조사", "별자리 이미지"]);
const CARAVAN_CAMEL: CategoryPath = CategoryPath::new(&["캐러밴", "낙타"]);
const CARAVAN_LEADER_PORTRAIT: CategoryPath = CategoryPath::new(&["캐러밴", "대장 초상화"]);

const fn in_game_range(start: u32, end: u32, category: CategoryPath) -> RecordRule {
    RecordRule::verified(
        RuleScope::BlockRange { start, end },
        category,
        CategorySource::InGame,
    )
}

const fn custom_range(start: u32, end: u32, category: CategoryPath) -> RecordRule {
    RecordRule::verified(
        RuleScope::BlockRange { start, end },
        category,
        CategorySource::Custom,
    )
}

pub(crate) const RECORD_RULES: &[RecordRule] = &[
    custom_range(0, 2_926, DISCOVERY_SET_1_LARGE),
    in_game_range(3_071, 3_288, BATTLE),
    custom_range(3_289, 3_314, PORT_PERMIT_ACQUISITION),
    in_game_range(3_315, 3_507, HELP),
    custom_range(4_023, 4_026, CLIENT_SPLASH_UI),
    custom_range(7_933, 8_718, HELP_SCREEN),
    in_game_range(8_842, 9_248, WORLD_MAP),
    in_game_range(9_291, 9_978, FIELD_MAP),
    custom_range(9_979, 10_011, DISCOVERY_SET_2_LARGE),
    in_game_range(10_012, 10_155, DUNGEON_MAP),
    in_game_range(10_156, 10_175, BLESSING),
    in_game_range(10_176, 10_199, EVENT),
    custom_range(10_200, 10_202, PERSON_UNCLASSIFIED_PORTRAIT),
    custom_range(10_203, 10_242, CLIENT_PLACEHOLDER),
    custom_range(10_243, 10_271, LEGEND_UNDISCOVERED),
    custom_range(10_272, 10_303, RESCUED_AIDE_PORTRAIT),
    custom_range(10_304, 10_367, RESCUED_AIDE_IMAGE),
    custom_range(10_368, 10_395, PROPHECY_BOOK_COVER),
    custom_range(10_396, 10_399, LEGACY_THEME_UI),
    custom_range(10_400, 10_418, LEGACY_DETAIL_UI),
    custom_range(10_439, 10_470, CONSTELLATION_SKY_MAP),
    custom_range(10_471, 10_543, CONSTELLATION_LINES),
    custom_range(10_544, 10_616, CONSTELLATION_LINES_AND_ART),
    custom_range(10_617, 10_800, CONSTELLATION_IMAGE),
    custom_range(10_801, 10_811, PERSON_UNCLASSIFIED_PORTRAIT),
    custom_range(10_812, 10_821, CARAVAN_CAMEL),
    custom_range(10_822, 10_830, CARAVAN_LEADER_PORTRAIT),
];
