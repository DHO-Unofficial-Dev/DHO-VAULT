// SPDX-License-Identifier: MPL-2.0

use dho_core::{
    DtMasterData, DtMasterFieldKind as MasterField, DtMasterParseError, DtMasterTableSpec,
    DtMasterValue, DtTextParseError, DtTextTable, IndexParseError, IndexedArchive,
    LicenseTermsArchive,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const TEXT_PAGE_SIZE: usize = 50;
pub const DEFAULT_TEXT_LANGUAGE_BLOCK: usize = 1;
// Version 2 selects the verified Korean language block instead of the Japanese base block.
pub const TEXT_SNAPSHOT_FORMAT_VERSION: u32 = 2;
pub const TEXT_IMAGE_RELATION_SNAPSHOT_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy)]
struct TextSourceDefinition {
    file_name: &'static str,
    label: &'static str,
    description: &'static str,
}

const TEXT_SOURCES: [TextSourceDefinition; 13] = [
    TextSourceDefinition {
        file_name: "dt000000.bin",
        label: "장소·기본 텍스트",
        description: "장소 이름과 기본 자료",
    },
    TextSourceDefinition {
        file_name: "dt000002.bin",
        label: "시스템 문구",
        description: "메뉴·상태·안내 문구",
    },
    TextSourceDefinition {
        file_name: "dt000003.bin",
        label: "이벤트 회상",
        description: "국가별·세계일주 이벤트 요약",
    },
    TextSourceDefinition {
        file_name: "dt000004.bin",
        label: "원시 관계표",
        description: "명령 표와 건수가 같은 3×u32 레코드 · 의미 미확정",
    },
    TextSourceDefinition {
        file_name: "dt000005.bin",
        label: "이벤트·임무",
        description: "이벤트 대사, 임무 진행 문구와 내부 명칭",
    },
    TextSourceDefinition {
        file_name: "dt000100.bin",
        label: "국가 이벤트 공통",
        description: "국가 이벤트 계열의 공통·예약 자료",
    },
    TextSourceDefinition {
        file_name: "dt000101.bin",
        label: "에스파냐 국가 이벤트",
        description: "에스파냐 초기 진행과 국가 이벤트 문구",
    },
    TextSourceDefinition {
        file_name: "dt000102.bin",
        label: "포르투갈 국가 이벤트",
        description: "포르투갈 초기 진행과 국가 이벤트 문구",
    },
    TextSourceDefinition {
        file_name: "dt000103.bin",
        label: "베네치아 국가 이벤트",
        description: "베네치아 초기 진행과 국가 이벤트 문구",
    },
    TextSourceDefinition {
        file_name: "dt000104.bin",
        label: "프랑스 국가 이벤트",
        description: "프랑스 초기 진행과 국가 이벤트 문구",
    },
    TextSourceDefinition {
        file_name: "dt000105.bin",
        label: "네덜란드 국가 이벤트",
        description: "네덜란드 초기 진행과 국가 이벤트 문구",
    },
    TextSourceDefinition {
        file_name: "dt000106.bin",
        label: "잉글랜드 국가 이벤트",
        description: "잉글랜드 초기 진행과 국가 이벤트 문구",
    },
    TextSourceDefinition {
        file_name: "dt000107.bin",
        label: "국가 이벤트 7",
        description: "국가 이벤트 계열의 추가·예약 자료",
    },
];

#[derive(Debug, Clone, Copy)]
enum MasterImageRule {
    None,
    Fixed {
        archive: &'static str,
        group_code: u32,
        relation: &'static str,
    },
    FixedOffset {
        archive: &'static str,
        group_code: u32,
        id_offset: u32,
        relation: &'static str,
    },
    IdDerivedGroup {
        archive: &'static str,
        divisor: u32,
        relation: &'static str,
    },
    Discovery,
    LegendDiscovery,
}

#[derive(Debug, Clone, Copy)]
struct MasterSourceDefinition {
    key: &'static str,
    label: &'static str,
    description: &'static str,
    table: DtMasterTableSpec,
    image_rule: MasterImageRule,
}

const TEXT_TEXT: &[MasterField] = &[MasterField::Text, MasterField::Text];
const TEXT_ONLY: &[MasterField] = &[MasterField::Text];
const COMMAND_FIELDS: &[MasterField] = &[MasterField::Text, MasterField::U32, MasterField::U32];
const PROFESSION_FIELDS: &[MasterField] = &[MasterField::Text, MasterField::Text, MasterField::U16];
const RANK_FIELDS: &[MasterField] = &[MasterField::Text, MasterField::Text, MasterField::Text];
const OCEAN_FIELDS: &[MasterField] = &[MasterField::Text, MasterField::U32];
const CITY_FIELDS: &[MasterField] = &[
    MasterField::Text,
    MasterField::U32,
    MasterField::U32,
    MasterField::U8,
];
const SUBURB_FIELDS: &[MasterField] = &[MasterField::Text, MasterField::U8, MasterField::U8];
const SKILL_FIELDS: &[MasterField] = &[
    MasterField::Text,
    MasterField::Text,
    MasterField::U16,
    MasterField::U16,
    MasterField::U8,
    MasterField::U8,
    MasterField::U16,
    MasterField::U16,
    MasterField::U16,
    MasterField::U32,
    MasterField::U16,
];
const EQUIPMENT_FIELDS: &[MasterField] = &[
    MasterField::Text,
    MasterField::Text,
    MasterField::U16,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
];
const TRADE_GOOD_FIELDS: &[MasterField] = &[MasterField::Text, MasterField::Text, MasterField::U16];
const CANNON_FIELDS: &[MasterField] = &[
    MasterField::Text,
    MasterField::Text,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
];
const ADDITIONAL_ARMOR_FIELDS: &[MasterField] = &[
    MasterField::Text,
    MasterField::Text,
    MasterField::U16,
    MasterField::U16,
    MasterField::U16,
    MasterField::U16,
];
const SPECIAL_EQUIPMENT_FIELDS: &[MasterField] = &[
    MasterField::Text,
    MasterField::Text,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
    MasterField::U8,
    MasterField::U8,
    MasterField::U8,
    MasterField::U8,
    MasterField::U8,
    MasterField::U8,
    MasterField::U8,
    MasterField::U8,
    MasterField::U8,
    MasterField::U8,
];
const FOUR_U32_FIELDS: &[MasterField] = &[
    MasterField::Text,
    MasterField::Text,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
];
const FIVE_U32_FIELDS: &[MasterField] = &[
    MasterField::Text,
    MasterField::Text,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
];
const SHIP_FIELDS: &[MasterField] = &[
    MasterField::Text,
    MasterField::Text,
    MasterField::U16,
    MasterField::U16,
    MasterField::U16,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
    MasterField::Skip(24),
];
const DISCOVERY_FIELDS: &[MasterField] = &[
    MasterField::Text,
    MasterField::Text,
    MasterField::U16,
    MasterField::U16,
    MasterField::U16,
    MasterField::U16,
    MasterField::U16,
    MasterField::U16,
    MasterField::U16,
];

const TEXT3: &[MasterField] = &[MasterField::Text, MasterField::Text, MasterField::Text];
const TEXT5: &[MasterField] = &[
    MasterField::Text,
    MasterField::Text,
    MasterField::Text,
    MasterField::Text,
    MasterField::Text,
];
const TEXT_U16: &[MasterField] = &[MasterField::Text, MasterField::U16];
const TEXT_TEXT_U8: &[MasterField] = &[MasterField::Text, MasterField::Text, MasterField::U8];
const TEXT_TEXT_U16: &[MasterField] = &[MasterField::Text, MasterField::Text, MasterField::U16];
const TEXT_U32_U8: &[MasterField] = &[MasterField::Text, MasterField::U32, MasterField::U8];
const TEXT_U32_U32: &[MasterField] = &[MasterField::Text, MasterField::U32, MasterField::U32];
const TEXT_EIGHT_U32: &[MasterField] = &[
    MasterField::Text,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
    MasterField::U32,
];
const TEXT_U32_SKIP_32: &[MasterField] =
    &[MasterField::Text, MasterField::U32, MasterField::Skip(32)];
const TECHNIQUE_FIELDS: &[MasterField] = &[
    MasterField::Text,
    MasterField::Text,
    MasterField::U16,
    MasterField::U16,
    MasterField::U16,
    MasterField::U8,
    MasterField::U8,
    MasterField::U8,
    MasterField::U8,
    MasterField::U8,
    MasterField::U16,
];
const SKIP_39_TEXT_TEXT: &[MasterField] =
    &[MasterField::Skip(39), MasterField::Text, MasterField::Text];
const SKIP_15_TEXT_TEXT: &[MasterField] =
    &[MasterField::Skip(15), MasterField::Text, MasterField::Text];
const SKIP_19_TEXT: &[MasterField] = &[MasterField::Skip(19), MasterField::Text];
const TEXT_TEXT_SKIP_5: &[MasterField] =
    &[MasterField::Text, MasterField::Text, MasterField::Skip(5)];
const TEXT5_SKIP_6: &[MasterField] = &[
    MasterField::Text,
    MasterField::Text,
    MasterField::Text,
    MasterField::Text,
    MasterField::Text,
    MasterField::Skip(6),
];
const TEXT_TEXT_SKIP_70: &[MasterField] =
    &[MasterField::Text, MasterField::Text, MasterField::Skip(70)];
const TEXT_TEXT_SKIP_1: &[MasterField] =
    &[MasterField::Text, MasterField::Text, MasterField::Skip(1)];
const TEXT_TEXT_SKIP_2: &[MasterField] =
    &[MasterField::Text, MasterField::Text, MasterField::Skip(2)];
const TEXT_SKIP_5: &[MasterField] = &[MasterField::Text, MasterField::Skip(5)];
const TEXT_U8: &[MasterField] = &[MasterField::Text, MasterField::U8];
const LEGEND_FIELDS: &[MasterField] = &[
    MasterField::Text,
    MasterField::Text,
    MasterField::U16,
    MasterField::Skip(78),
];
const TEXT_U8_U8: &[MasterField] = &[MasterField::Text, MasterField::U8, MasterField::U8];
const TEXT_TEXT_SKIP_10: &[MasterField] =
    &[MasterField::Text, MasterField::Text, MasterField::Skip(10)];
const CREW_EQUIPMENT_FIELDS: &[MasterField] = &[
    MasterField::Text,
    MasterField::Text,
    MasterField::U8,
    MasterField::U8,
    MasterField::U8,
    MasterField::U8,
    MasterField::U8,
    MasterField::U8,
    MasterField::U8,
    MasterField::Skip(13),
    MasterField::U8,
    MasterField::U8,
    MasterField::U8,
    MasterField::U8,
    MasterField::U8,
    MasterField::U8,
];

macro_rules! extra_master_source {
    ($key:literal, $label:literal, $offset:literal, $fields:expr) => {
        MasterSourceDefinition {
            key: $key,
            label: $label,
            description: "한국 클라이언트에서 레코드 경계를 검증한 추가 마스터 문자열",
            table: DtMasterTableSpec {
                directory_offset: $offset,
                fields: $fields,
            },
            image_rule: MasterImageRule::None,
        }
    };
}

const MASTER_SOURCES: [MasterSourceDefinition; 30] = [
    MasterSourceDefinition {
        key: "command",
        label: "명령",
        description: "게임 명령 이름과 원시 참조 값",
        table: DtMasterTableSpec {
            directory_offset: 0x04,
            fields: COMMAND_FIELDS,
        },
        image_rule: MasterImageRule::None,
    },
    MasterSourceDefinition {
        key: "region",
        label: "지역",
        description: "명산품 판정 등에 쓰이는 지역 이름",
        table: DtMasterTableSpec {
            directory_offset: 0x0c,
            fields: TEXT_ONLY,
        },
        image_rule: MasterImageRule::None,
    },
    MasterSourceDefinition {
        key: "country",
        label: "국가",
        description: "국가 이름과 설명",
        table: DtMasterTableSpec {
            directory_offset: 0x24,
            fields: TEXT_TEXT,
        },
        image_rule: MasterImageRule::None,
    },
    MasterSourceDefinition {
        key: "profession",
        label: "직업",
        description: "직업 이름과 설명 및 원시 유형 값",
        table: DtMasterTableSpec {
            directory_offset: 0x2c,
            fields: PROFESSION_FIELDS,
        },
        image_rule: MasterImageRule::None,
    },
    MasterSourceDefinition {
        key: "skill",
        label: "스킬",
        description: "스킬 이름과 설명",
        table: DtMasterTableSpec {
            directory_offset: 0x34,
            fields: SKILL_FIELDS,
        },
        image_rule: MasterImageRule::Fixed {
            archive: "sa",
            group_code: 0,
            relation: "스킬 아이콘",
        },
    },
    MasterSourceDefinition {
        key: "rank",
        label: "작위",
        description: "일반·오스만·왕립함대 작위 명칭",
        table: DtMasterTableSpec {
            directory_offset: 0x3c,
            fields: RANK_FIELDS,
        },
        image_rule: MasterImageRule::None,
    },
    MasterSourceDefinition {
        key: "ocean",
        label: "해역",
        description: "해역 이름과 해역군 참조 값",
        table: DtMasterTableSpec {
            directory_offset: 0x44,
            fields: OCEAN_FIELDS,
        },
        image_rule: MasterImageRule::None,
    },
    MasterSourceDefinition {
        key: "ocean_group",
        label: "해역군",
        description: "해역군 이름",
        table: DtMasterTableSpec {
            directory_offset: 0x4c,
            fields: TEXT_ONLY,
        },
        image_rule: MasterImageRule::None,
    },
    MasterSourceDefinition {
        key: "city",
        label: "도시",
        description: "도시 이름과 도시 유형·소속 국가·문화권 참조 값",
        table: DtMasterTableSpec {
            directory_offset: 0x54,
            fields: CITY_FIELDS,
        },
        image_rule: MasterImageRule::None,
    },
    MasterSourceDefinition {
        key: "suburb",
        label: "교외",
        description: "교외 이름과 소속 도시·문화권 참조 값",
        table: DtMasterTableSpec {
            directory_offset: 0x5c,
            fields: SUBURB_FIELDS,
        },
        image_rule: MasterImageRule::None,
    },
    MasterSourceDefinition {
        key: "item_major_category",
        label: "물품 대분류",
        description: "물품 대분류 이름",
        table: DtMasterTableSpec {
            directory_offset: 0x64,
            fields: TEXT_ONLY,
        },
        image_rule: MasterImageRule::None,
    },
    MasterSourceDefinition {
        key: "item_minor_category",
        label: "물품 소분류",
        description: "물품 소분류 이름",
        table: DtMasterTableSpec {
            directory_offset: 0x6c,
            fields: TEXT_ONLY,
        },
        image_rule: MasterImageRule::None,
    },
    MasterSourceDefinition {
        key: "item",
        label: "아이템",
        description: "아이템 이름과 설명",
        table: DtMasterTableSpec {
            directory_offset: 0x74,
            fields: TEXT_TEXT,
        },
        image_rule: MasterImageRule::IdDerivedGroup {
            archive: "sb",
            divisor: 100_000,
            relation: "아이템 이미지",
        },
    },
    MasterSourceDefinition {
        key: "equipment",
        label: "장비",
        description: "장비 이름과 설명",
        table: DtMasterTableSpec {
            directory_offset: 0x7c,
            fields: EQUIPMENT_FIELDS,
        },
        image_rule: MasterImageRule::IdDerivedGroup {
            archive: "sb",
            divisor: 100_000,
            relation: "장비 이미지",
        },
    },
    MasterSourceDefinition {
        key: "recipe",
        label: "생산 레시피",
        description: "생산 레시피 이름과 설명 · 이미지 직접 연결 없음",
        table: DtMasterTableSpec {
            directory_offset: 0x84,
            fields: TEXT_TEXT,
        },
        image_rule: MasterImageRule::None,
    },
    MasterSourceDefinition {
        key: "map",
        label: "지도",
        description: "지도 아이템 이름과 설명",
        table: DtMasterTableSpec {
            directory_offset: 0x8c,
            fields: TEXT_TEXT,
        },
        image_rule: MasterImageRule::IdDerivedGroup {
            archive: "sb",
            divisor: 100_000,
            relation: "지도 이미지",
        },
    },
    MasterSourceDefinition {
        key: "trade_good_category",
        label: "교역품 유형",
        description: "교역품 유형 이름",
        table: DtMasterTableSpec {
            directory_offset: 0x94,
            fields: TEXT_ONLY,
        },
        image_rule: MasterImageRule::None,
    },
    MasterSourceDefinition {
        key: "trade_good",
        label: "교역품",
        description: "교역품 이름과 설명",
        table: DtMasterTableSpec {
            directory_offset: 0x9c,
            fields: TRADE_GOOD_FIELDS,
        },
        image_rule: MasterImageRule::Fixed {
            archive: "sc",
            group_code: 0,
            relation: "교역품 이미지",
        },
    },
    MasterSourceDefinition {
        key: "decoration",
        label: "장식품",
        description: "장식품 이름과 설명",
        table: DtMasterTableSpec {
            directory_offset: 0xa4,
            fields: TEXT_TEXT,
        },
        image_rule: MasterImageRule::IdDerivedGroup {
            archive: "sb",
            divisor: 100_000,
            relation: "장식품 이미지",
        },
    },
    MasterSourceDefinition {
        key: "cannonball",
        label: "포탄",
        description: "포탄 유형 이름",
        table: DtMasterTableSpec {
            directory_offset: 0xac,
            fields: TEXT_ONLY,
        },
        image_rule: MasterImageRule::None,
    },
    MasterSourceDefinition {
        key: "cannon",
        label: "대포",
        description: "선박 대포 이름과 설명",
        table: DtMasterTableSpec {
            directory_offset: 0xb4,
            fields: CANNON_FIELDS,
        },
        image_rule: MasterImageRule::Fixed {
            archive: "sb",
            group_code: 7,
            relation: "대포 이미지",
        },
    },
    MasterSourceDefinition {
        key: "additional_armor",
        label: "추가 장갑",
        description: "추가 장갑 이름과 설명",
        table: DtMasterTableSpec {
            directory_offset: 0xbc,
            fields: ADDITIONAL_ARMOR_FIELDS,
        },
        image_rule: MasterImageRule::Fixed {
            archive: "sb",
            group_code: 8,
            relation: "추가 장갑 이미지",
        },
    },
    MasterSourceDefinition {
        key: "special_equipment",
        label: "특수 장비",
        description: "특수 선박 장비 이름과 설명",
        table: DtMasterTableSpec {
            directory_offset: 0xc4,
            fields: SPECIAL_EQUIPMENT_FIELDS,
        },
        image_rule: MasterImageRule::Fixed {
            archive: "sb",
            group_code: 9,
            relation: "특수 장비 이미지",
        },
    },
    MasterSourceDefinition {
        key: "aux_sail",
        label: "보조돛",
        description: "보조돛 이름과 설명",
        table: DtMasterTableSpec {
            directory_offset: 0xcc,
            fields: FOUR_U32_FIELDS,
        },
        image_rule: MasterImageRule::Fixed {
            archive: "sb",
            group_code: 6,
            relation: "보조돛 이미지",
        },
    },
    MasterSourceDefinition {
        key: "emblem",
        label: "선박 문장",
        description: "선박 문장 이름과 설명",
        table: DtMasterTableSpec {
            directory_offset: 0xd4,
            fields: TEXT_TEXT,
        },
        image_rule: MasterImageRule::Fixed {
            archive: "sb",
            group_code: 11,
            relation: "문장 이미지",
        },
    },
    MasterSourceDefinition {
        key: "figurehead",
        label: "선수상",
        description: "선수상 이름과 설명",
        table: DtMasterTableSpec {
            directory_offset: 0xdc,
            fields: FIVE_U32_FIELDS,
        },
        image_rule: MasterImageRule::Fixed {
            archive: "sb",
            group_code: 10,
            relation: "선수상 이미지",
        },
    },
    MasterSourceDefinition {
        key: "ship",
        label: "선박",
        description: "선박 이름과 설명",
        table: DtMasterTableSpec {
            directory_offset: 0xe4,
            fields: SHIP_FIELDS,
        },
        image_rule: MasterImageRule::Fixed {
            archive: "sc",
            group_code: 1,
            relation: "선박 표시 이미지",
        },
    },
    MasterSourceDefinition {
        key: "sail_rig",
        label: "범장",
        description: "마스트 수와 가로돛·세로돛 구성 방식 · 직접 이미지 연결 없음",
        table: DtMasterTableSpec {
            directory_offset: 0xec,
            fields: TEXT_TEXT,
        },
        image_rule: MasterImageRule::None,
    },
    MasterSourceDefinition {
        key: "armor_texture",
        label: "선박 재질",
        description: "선박 재질 이름과 설명",
        table: DtMasterTableSpec {
            directory_offset: 0xf4,
            fields: TEXT_TEXT,
        },
        image_rule: MasterImageRule::Fixed {
            archive: "sc",
            group_code: 2,
            relation: "선박 재질 이미지",
        },
    },
    MasterSourceDefinition {
        key: "discovery",
        label: "발견물",
        description: "발견물 이름과 설명 · 목록/획득 이미지 연결",
        table: DtMasterTableSpec {
            directory_offset: 0x104,
            fields: DISCOVERY_FIELDS,
        },
        image_rule: MasterImageRule::Discovery,
    },
];

const EXTRA_MASTER_SOURCES: &[MasterSourceDefinition] = &[
    extra_master_source!("discovery_type", "발견물 유형", 0x0fc, TEXT_ONLY),
    extra_master_source!("card_attribute", "카드 속성", 0x10c, TEXT_ONLY),
    extra_master_source!("tarot_card", "타로 카드", 0x114, TEXT_TEXT),
    extra_master_source!("title", "칭호", 0x11c, TEXT_TEXT),
    extra_master_source!("aide_post", "부관 담당", 0x124, TEXT_ONLY),
    extra_master_source!("tavern_menu", "주점 메뉴", 0x12c, TEXT_TEXT_U16),
    extra_master_source!("place", "장소", 0x13c, TEXT_U16),
    extra_master_source!("building", "건물", 0x144, TEXT_ONLY),
    extra_master_source!("liner", "정기선", 0x14c, TEXT_U32_U32),
    extra_master_source!("region_detail", "지역 상세", 0x15c, TEXT_EIGHT_U32),
    extra_master_source!("private_farm_land", "개인 농장 토지", 0x16c, TEXT_ONLY),
    extra_master_source!("private_farm_facility", "개인 농장 시설", 0x174, TEXT_ONLY),
    extra_master_source!("pet", "애완동물", 0x17c, TEXT_U32_SKIP_32),
    extra_master_source!("experiment_furnace", "실험로", 0x18c, TEXT_TEXT),
    extra_master_source!("special_item", "특수 물품", 0x194, TEXT_TEXT),
    MasterSourceDefinition {
        key: "technique",
        label: "테크닉",
        description: "테크닉 이름, 설명과 수치 · 테크닉 이미지 연결",
        table: DtMasterTableSpec {
            directory_offset: 0x19c,
            fields: TECHNIQUE_FIELDS,
        },
        image_rule: MasterImageRule::Fixed {
            archive: "sc",
            group_code: 16,
            relation: "테크닉 이미지",
        },
    },
    extra_master_source!("dungeon", "던전", 0x1ac, TEXT_ONLY),
    extra_master_source!("memorial_album", "메모리얼 앨범", 0x1b4, TEXT_TEXT),
    extra_master_source!("colony_building", "개척지 건물", 0x1c4, TEXT_TEXT_U16),
    extra_master_source!("colony_decoration", "개척지 장식", 0x1d4, TEXT_ONLY),
    extra_master_source!("colony_port", "개척항", 0x1dc, TEXT_ONLY),
    extra_master_source!(
        "original_ougi_prefix",
        "오리지널 오의 명칭 1",
        0x1e4,
        TEXT_U32_U8
    ),
    extra_master_source!(
        "original_ougi_suffix",
        "오리지널 오의 명칭 2",
        0x1ec,
        TEXT_U32_U8
    ),
    extra_master_source!("university_subject", "대학 과제", 0x204, TEXT_TEXT),
    MasterSourceDefinition {
        key: "university_research",
        label: "대학 연구",
        description: "대학·학술협회 연구 이름과 설명 · 연구 이미지 연결",
        table: DtMasterTableSpec {
            directory_offset: 0x20c,
            fields: SKIP_39_TEXT_TEXT,
        },
        image_rule: MasterImageRule::Fixed {
            archive: "sc",
            group_code: 20,
            relation: "대학·학술협회 연구 이미지",
        },
    },
    extra_master_source!(
        "university_theme",
        "대학 연구 주제",
        0x214,
        SKIP_15_TEXT_TEXT
    ),
    extra_master_source!(
        "university_navigation",
        "대학 항해 연구",
        0x21c,
        SKIP_19_TEXT
    ),
    extra_master_source!("option_service", "옵션 서비스", 0x224, TEXT_TEXT_U8),
    extra_master_source!("information_service", "정보편", 0x22c, TEXT_TEXT_SKIP_5),
    extra_master_source!("placeholder_text", "예약 다중 문구", 0x234, TEXT5_SKIP_6),
    extra_master_source!("historical_person", "가나돌", 0x244, TEXT_TEXT),
    extra_master_source!("office_self", "호칭 효과", 0x24c, TEXT_TEXT),
    extra_master_source!("office_party", "호칭 함대 효과", 0x254, TEXT_TEXT),
    extra_master_source!("emperor_combo", "황제 콤보", 0x25c, TEXT_ONLY),
    extra_master_source!("emperor", "황제", 0x264, TEXT_ONLY),
    extra_master_source!("emperor_option", "황제 선택지", 0x26c, TEXT_TEXT),
    extra_master_source!("imperial_reward", "문무 보상", 0x274, TEXT_ONLY),
    extra_master_source!("emperor_skill", "황제 스킬", 0x27c, TEXT_TEXT),
    extra_master_source!("elector", "선제후", 0x284, TEXT_ONLY),
    extra_master_source!(
        "treasure_theme",
        "트레저 헌트 후보",
        0x28c,
        TEXT_TEXT_SKIP_70
    ),
    extra_master_source!("treasure_clue_text", "트레저 헌트 단서", 0x294, TEXT_ONLY),
    MasterSourceDefinition {
        key: "treasure_category",
        label: "트레저 헌트 테마",
        description: "트레저 헌트 테마 이름과 설명 · 테마 이미지 연결",
        table: DtMasterTableSpec {
            directory_offset: 0x29c,
            fields: TEXT_TEXT,
        },
        image_rule: MasterImageRule::Fixed {
            archive: "sc",
            group_code: 26,
            relation: "트레저 헌트 테마 이미지",
        },
    },
    MasterSourceDefinition {
        key: "treasure_relic",
        label: "트레저 헌트 렐릭",
        description: "트레저 헌트 렐릭 이름과 설명 · 동일 ID가 존재할 때 렐릭 이미지 연결",
        table: DtMasterTableSpec {
            directory_offset: 0x2a4,
            fields: TEXT_TEXT,
        },
        image_rule: MasterImageRule::Fixed {
            archive: "sc",
            group_code: 32,
            relation: "트레저 헌트 렐릭 이미지",
        },
    },
    extra_master_source!("emperor_period", "황제 시대", 0x2ac, TEXT_TEXT_SKIP_1),
    extra_master_source!("emperor_effect", "황제 효과", 0x2b4, TEXT_ONLY),
    extra_master_source!("emperor_event", "황제 이벤트", 0x2bc, TEXT5),
    extra_master_source!("mentor_guide", "멘토 가이드", 0x2c4, TEXT_ONLY),
    extra_master_source!("ship_grade_type", "선박 그레이드 유형", 0x2cc, TEXT_TEXT),
    extra_master_source!("ship_grade_skill", "선박 그레이드 스킬", 0x2d4, TEXT_TEXT),
    extra_master_source!("historical_period", "시대", 0x2dc, TEXT_TEXT_SKIP_1),
    extra_master_source!("stage_title", "스테이지 칭호", 0x2e4, TEXT_TEXT_SKIP_2),
    extra_master_source!("help_topic", "도움말", 0x2ec, TEXT_TEXT),
    extra_master_source!("pirate_title", "해적 호칭", 0x2f4, TEXT_TEXT),
    extra_master_source!("title_prefix", "별칭 접두어", 0x2fc, TEXT_SKIP_5),
    extra_master_source!("title_suffix", "별칭 접미어", 0x304, TEXT_SKIP_5),
    extra_master_source!("sea_survey_guide", "해역 조사 설명", 0x30c, TEXT_ONLY),
    extra_master_source!("city_npc_type", "도시 NPC 유형", 0x314, TEXT_ONLY),
    extra_master_source!("information_type", "정보 유형", 0x31c, TEXT_ONLY),
    extra_master_source!("city_category", "도시 분류", 0x324, TEXT_ONLY),
    extra_master_source!("land_survey_goal", "육지 조사 목표", 0x36c, TEXT_ONLY),
    extra_master_source!("transmutation_alchemy", "변성 연금", 0x374, TEXT_TEXT),
    extra_master_source!("fleet_dispatch_region", "파견 지역", 0x384, TEXT_ONLY),
    extra_master_source!("regional_fleet_event", "지방 함대 이벤트", 0x38c, TEXT3),
    extra_master_source!("railway_company", "철도 회사", 0x394, TEXT_TEXT),
    extra_master_source!("railway_investment", "철도 투자 효과", 0x39c, TEXT_TEXT),
    extra_master_source!("daily_dialogue", "안내 대화", 0x3a4, TEXT_ONLY),
    extra_master_source!("beginner_log", "세일러즈 가이드", 0x3b4, TEXT_TEXT),
    extra_master_source!("beginner_tutorial", "초보자 튜토리얼", 0x3bc, TEXT_TEXT),
    extra_master_source!("historical_npc", "가나돌 NPC", 0x3c4, TEXT_U8),
    extra_master_source!("voyage_log", "항해 로그", 0x3cc, TEXT3),
    extra_master_source!("voyage_guide", "항해 가이드", 0x3d4, TEXT3),
    extra_master_source!("guide_category", "가이드 분류", 0x3dc, TEXT_ONLY),
    MasterSourceDefinition {
        key: "legend_discovery",
        label: "전승 발견",
        description: "전승 이름과 설명 · 획득 아이콘과 큰 이미지 연결",
        table: DtMasterTableSpec {
            directory_offset: 0x3e4,
            fields: LEGEND_FIELDS,
        },
        image_rule: MasterImageRule::LegendDiscovery,
    },
    extra_master_source!("mysterious_sea", "이상한 바다", 0x3ec, LEGEND_FIELDS),
    extra_master_source!("fleet_obstruction", "함대 방해 효과", 0x3f4, TEXT_TEXT),
    extra_master_source!("thank_you_letter", "감사장 효과", 0x414, TEXT_TEXT),
    extra_master_source!("rescue_npc", "구조 NPC", 0x41c, TEXT_U8_U8),
    extra_master_source!("legacy_information", "레거시 정보", 0x42c, TEXT_TEXT),
    MasterSourceDefinition {
        key: "legacy_theme",
        label: "레거시 테마",
        description: "레거시 테마 이름과 설명 · 테마 이미지 연결",
        table: DtMasterTableSpec {
            directory_offset: 0x434,
            fields: TEXT_TEXT,
        },
        image_rule: MasterImageRule::Fixed {
            archive: "sc",
            group_code: 33,
            relation: "레거시 테마 이미지",
        },
    },
    MasterSourceDefinition {
        key: "pursuit_production",
        label: "추구 생산",
        description: "추구 생산 이름과 설명 · 추구 생산 이미지 연결",
        table: DtMasterTableSpec {
            directory_offset: 0x43c,
            fields: TEXT_TEXT,
        },
        image_rule: MasterImageRule::Fixed {
            archive: "sc",
            group_code: 34,
            relation: "추구 생산 이미지",
        },
    },
    extra_master_source!("training_experience", "체험 훈련", 0x444, TEXT_TEXT),
    MasterSourceDefinition {
        key: "ship_decoration",
        label: "선박 데코",
        description: "선박 데코 이름과 설명 · 선박 데코 이미지 연결",
        table: DtMasterTableSpec {
            directory_offset: 0x454,
            fields: TEXT_TEXT_SKIP_10,
        },
        image_rule: MasterImageRule::Fixed {
            archive: "sb",
            group_code: 25,
            relation: "선박 데코 이미지",
        },
    },
    MasterSourceDefinition {
        key: "crew_equipment",
        label: "선원 장비",
        description: "선원 장비 이름, 설명과 수치 · 선원 장비 이미지 연결",
        table: DtMasterTableSpec {
            directory_offset: 0x45c,
            fields: CREW_EQUIPMENT_FIELDS,
        },
        image_rule: MasterImageRule::Fixed {
            archive: "sb",
            group_code: 26,
            relation: "선원 장비 이미지",
        },
    },
    extra_master_source!("magic_attribute", "마법 속성", 0x464, TEXT_ONLY),
    extra_master_source!("constellation", "별자리 조사", 0x474, TEXT_ONLY),
    extra_master_source!("constellation_intro", "별자리 소개", 0x484, TEXT_ONLY),
    extra_master_source!("constellation_reward", "별자리 조사 보상", 0x48c, TEXT_ONLY),
    MasterSourceDefinition {
        key: "biography",
        label: "위인의 장",
        description: "위인의 장 테마 이름과 설명 · 공통 아이콘 다음의 테마 이미지 연결",
        table: DtMasterTableSpec {
            directory_offset: 0x49c,
            fields: TEXT_TEXT,
        },
        image_rule: MasterImageRule::FixedOffset {
            archive: "sc",
            group_code: 35,
            id_offset: 1,
            relation: "위인의 장 테마 이미지",
        },
    },
    extra_master_source!("biography_chapter", "위인의 장 챕터", 0x4a4, TEXT_ONLY),
    MasterSourceDefinition {
        key: "biography_intro",
        label: "위인의 장 인트로",
        description: "위인 이름과 소개 · 공통 아이콘 다음의 테마 이미지 연결",
        table: DtMasterTableSpec {
            directory_offset: 0x4ac,
            fields: TEXT_TEXT,
        },
        image_rule: MasterImageRule::FixedOffset {
            archive: "sc",
            group_code: 35,
            id_offset: 1,
            relation: "위인의 장 인물 이미지",
        },
    },
    extra_master_source!("gemstone_intro", "잠재능력", 0x4b4, TEXT_TEXT),
    extra_master_source!("caravan_person", "캐러밴 인물", 0x4bc, TEXT_TEXT),
    extra_master_source!("overland_effect", "육로 이동 효과", 0x4c4, TEXT_TEXT),
];

#[derive(Debug, Clone)]
struct CatalogTextRecord {
    source: String,
    source_label: String,
    id: u32,
    fields: Vec<String>,
    image_links: Vec<TextImageLink>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExpectedImageLink {
    archive: String,
    group_code: u32,
    icon_id: u32,
    relation: String,
}

#[derive(Debug, Default)]
struct ImageLinkIndex {
    records: BTreeMap<(String, u32, u32), u32>,
}

impl ImageLinkIndex {
    fn load(game_directory: &Path) -> Result<Self, TextCatalogError> {
        let directory = game_directory.join("0010").join("0001");
        let mut records = BTreeMap::new();
        for archive in ["sa", "sb", "sc", "sd"] {
            let path = directory.join(format!("{archive}000000.bin"));
            if !path.is_file() {
                continue;
            }
            let bytes = fs::read(&path).map_err(|source| TextCatalogError::ReadImageIndex {
                path: path.clone(),
                source,
            })?;
            let index = IndexedArchive::parse(&bytes).map_err(|source| {
                TextCatalogError::ParseImageIndex {
                    path: path.clone(),
                    source,
                }
            })?;
            for record in index.records {
                records.insert(
                    (archive.to_owned(), record.group_code, record.icon_id),
                    record.block_index,
                );
            }
        }
        Ok(Self { records })
    }

    fn link(
        &self,
        archive: &str,
        group_code: u32,
        icon_id: u32,
        relation: &str,
    ) -> Option<TextImageLink> {
        self.records
            .get(&(archive.to_owned(), group_code, icon_id))
            .copied()
            .map(|block_index| TextImageLink {
                archive: archive.to_owned(),
                group_code,
                icon_id,
                block_index,
                relation: relation.to_owned(),
                evidence_type: TextImageRelationEvidence::MasterTypeRule,
                verification_status: TextImageRelationVerification::HumanVerified,
            })
    }
}

#[derive(Debug)]
pub struct TextCatalog {
    selected_language_block: usize,
    language_blocks: Vec<TextLanguageBlockSummary>,
    sources: Vec<TextSourceSummary>,
    records: Vec<CatalogTextRecord>,
    image_records: BTreeMap<(String, u32, u32), Vec<usize>>,
    image_group_sources: BTreeMap<(String, u32), String>,
}

impl TextCatalog {
    pub fn load(game_directory: impl AsRef<Path>) -> Result<Self, TextCatalogError> {
        Self::load_language_block(game_directory, DEFAULT_TEXT_LANGUAGE_BLOCK)
    }

    pub fn load_language_block(
        game_directory: impl AsRef<Path>,
        language_block_index: usize,
    ) -> Result<Self, TextCatalogError> {
        let game_directory = game_directory.as_ref();
        let local_directory = game_directory.join("0000").join("local");
        let mut sources = Vec::new();
        let mut records = Vec::new();
        let mut available_language_block_count = 1usize;
        let image_index = ImageLinkIndex::load(game_directory)?;

        let master_path = local_directory.join("dt000001.bin");
        if master_path.is_file() {
            let bytes = fs::read(&master_path).map_err(|source| TextCatalogError::Read {
                path: master_path.clone(),
                source,
            })?;
            let block_count = DtMasterData::block_count(&bytes).map_err(|source| {
                TextCatalogError::ParseMaster {
                    path: master_path.clone(),
                    source,
                }
            })?;
            available_language_block_count = available_language_block_count.max(block_count);
            let selected_block = selected_block_index(language_block_index, block_count)?;
            let master = DtMasterData::parse_block(&bytes, selected_block).map_err(|source| {
                TextCatalogError::ParseMaster {
                    path: master_path.clone(),
                    source,
                }
            })?;
            let mut trade_good_category_names = BTreeMap::new();
            let mut discovery_type_names = master
                .read_table_exact(DtMasterTableSpec {
                    directory_offset: 0x0fc,
                    fields: TEXT_ONLY,
                })
                .map(|table| {
                    table
                        .into_iter()
                        .filter_map(|record| match record.values.first() {
                            Some(DtMasterValue::Text(name)) => Some((record.id, name.clone())),
                            _ => None,
                        })
                        .collect::<BTreeMap<_, _>>()
                })
                .unwrap_or_default();
            for (definition, exact_boundary) in MASTER_SOURCES
                .iter()
                .map(|definition| (definition, false))
                .chain(
                    EXTRA_MASTER_SOURCES
                        .iter()
                        .map(|definition| (definition, true)),
                )
            {
                let table = if exact_boundary {
                    master.read_table_exact(definition.table)
                } else {
                    master.read_table(definition.table)
                };
                let source_key = format!("dt000001.bin:{}", definition.key);
                let table = match table {
                    Ok(table) => table,
                    Err(error) if exact_boundary => {
                        sources.push(TextSourceSummary {
                            file_name: source_key,
                            label: definition.label.to_owned(),
                            description: format!(
                                "{} · 현재 한국어 블록에서 구조 미지원: {error}",
                                definition.description
                            ),
                            record_count: 0,
                        });
                        continue;
                    }
                    Err(source) => {
                        return Err(TextCatalogError::ParseMaster {
                            path: master_path.clone(),
                            source,
                        });
                    }
                };
                let record_count = table.len();
                if definition.key == "trade_good_category" {
                    for record in &table {
                        if let Some(DtMasterValue::Text(name)) = record.values.first() {
                            trade_good_category_names.insert(record.id, name.clone());
                        }
                    }
                }
                if definition.key == "discovery_type" {
                    for record in &table {
                        if let Some(DtMasterValue::Text(name)) = record.values.first() {
                            discovery_type_names.insert(record.id, name.clone());
                        }
                    }
                }
                records.extend(table.into_iter().map(|record| {
                    let fields = master_value_fields(
                        definition.key,
                        &record.values,
                        &trade_good_category_names,
                        &discovery_type_names,
                    );
                    let image_links =
                        master_image_links(definition.image_rule, record.id, &image_index);
                    CatalogTextRecord {
                        source: source_key.clone(),
                        source_label: definition.label.to_owned(),
                        id: record.id,
                        fields,
                        image_links,
                    }
                }));
                sources.push(TextSourceSummary {
                    file_name: source_key,
                    label: definition.label.to_owned(),
                    description: format!(
                        "{} · {}",
                        definition.description,
                        language_block_label(selected_block)
                    ),
                    record_count,
                });
            }
            let action_source = "dt000001.bin:action_emotion".to_owned();
            match master.read_counted_u16_text_table_exact(0x134) {
                Ok(table) => {
                    let record_count = table.len();
                    records.extend(table.into_iter().map(|record| CatalogTextRecord {
                        source: action_source.clone(),
                        source_label: "행동·감정".to_owned(),
                        id: record.id,
                        fields: master_value_fields(
                            "action_emotion",
                            &record.values,
                            &trade_good_category_names,
                            &discovery_type_names,
                        ),
                        image_links: Vec::new(),
                    }));
                    sources.push(TextSourceSummary {
                        file_name: action_source,
                        label: "행동·감정".to_owned(),
                        description: format!(
                            "가변 길이 행동 이름과 내부 명령 · {}",
                            language_block_label(selected_block)
                        ),
                        record_count,
                    });
                }
                Err(error) => sources.push(TextSourceSummary {
                    file_name: action_source,
                    label: "행동·감정".to_owned(),
                    description: format!("가변 길이 행동 이름과 내부 명령 · 구조 미지원: {error}"),
                    record_count: 0,
                }),
            }
        }

        let license_path = local_directory.join("lc000000.bin");
        if license_path.is_file() {
            let bytes = fs::read(&license_path).map_err(|source| TextCatalogError::Read {
                path: license_path,
                source,
            })?;
            match LicenseTermsArchive::parse(&bytes) {
                Ok(archive) => {
                    let record_count = archive.blocks.len();
                    records.extend(archive.blocks.into_iter().map(|block| CatalogTextRecord {
                        source: "lc000000.bin".to_owned(),
                        source_label: "클라이언트 이용약관".to_owned(),
                        id: block.index as u32,
                        fields: vec![
                            block.identifier,
                            format!("컴파일된 16비트 단위: {}개", block.units.len()),
                            format!("원시 체크섬: {:016X}", terms_unit_hash(&block.units)),
                        ],
                        image_links: Vec::new(),
                    }));
                    sources.push(TextSourceSummary {
                        file_name: "lc000000.bin".to_owned(),
                        label: "클라이언트 이용약관".to_owned(),
                        description: "TERMS 검색표 메타데이터 · 이벤트·시스템 문장 자료가 아님"
                            .to_owned(),
                        record_count,
                    });
                }
                Err(error) => sources.push(TextSourceSummary {
                    file_name: "lc000000.bin".to_owned(),
                    label: "클라이언트 이용약관".to_owned(),
                    description: format!("TERMS 검색표 · 현재 구조 미지원: {error}"),
                    record_count: 0,
                }),
            }
        }

        let mut text_files = fs::read_dir(&local_directory)
            .map_err(|source| TextCatalogError::Read {
                path: local_directory.clone(),
                source,
            })?
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|file_name| {
                let normalized = file_name.to_ascii_lowercase();
                normalized.starts_with("dt")
                    && normalized.ends_with(".bin")
                    && normalized != "dt000001.bin"
            })
            .collect::<Vec<_>>();
        text_files.sort_by_key(|file_name| file_name.to_ascii_lowercase());

        for file_name in text_files {
            let definition = TEXT_SOURCES
                .iter()
                .find(|definition| definition.file_name.eq_ignore_ascii_case(&file_name));
            let label = definition
                .map(|definition| definition.label.to_owned())
                .unwrap_or_else(|| format!("DT 텍스트 · {file_name}"));
            let description = definition
                .map(|definition| definition.description.to_owned())
                .unwrap_or_else(|| "자동 발견한 클라이언트 문자열 자료".to_owned());
            let path = local_directory.join(&file_name);
            let bytes = fs::read(&path).map_err(|source| TextCatalogError::Read {
                path: path.clone(),
                source,
            })?;
            let block_count = match DtTextTable::block_count(&bytes) {
                Ok(block_count) => block_count,
                Err(error) => {
                    sources.push(TextSourceSummary {
                        file_name,
                        label,
                        description: format!("{description} · 현재 구조 미지원: {error}"),
                        record_count: 0,
                    });
                    continue;
                }
            };
            available_language_block_count = available_language_block_count.max(block_count);
            let selected_block = match selected_block_index(language_block_index, block_count) {
                Ok(selected_block) => selected_block,
                Err(error) => {
                    sources.push(TextSourceSummary {
                        file_name,
                        label,
                        description: format!("{description} · {error}"),
                        record_count: 0,
                    });
                    continue;
                }
            };
            let table = match DtTextTable::parse_block(&bytes, selected_block)
                .or_else(|_| DtTextTable::parse_fixed_rows_block(&bytes, selected_block))
                .or_else(|_| DtTextTable::parse_u32_rows_block(&bytes, selected_block))
            {
                Ok(table) => table,
                Err(error) => {
                    sources.push(TextSourceSummary {
                        file_name,
                        label,
                        description: format!("{description} · 현재 구조 미지원: {error}"),
                        record_count: 0,
                    });
                    continue;
                }
            };
            let record_count = table.records.len();
            records.extend(table.records.into_iter().map(|record| CatalogTextRecord {
                source: file_name.clone(),
                source_label: label.clone(),
                id: record.id,
                fields: record.fields,
                image_links: Vec::new(),
            }));
            sources.push(TextSourceSummary {
                file_name,
                label,
                description: format!(
                    "{description} · {} · 전체 {block_count}개",
                    language_block_label(selected_block)
                ),
                record_count,
            });
        }

        if sources.is_empty() {
            return Err(TextCatalogError::NoSupportedSources { local_directory });
        }
        let mut image_records = BTreeMap::<(String, u32, u32), Vec<usize>>::new();
        let mut image_group_source_sets = BTreeMap::<(String, u32), BTreeSet<String>>::new();
        for (record_index, record) in records.iter().enumerate() {
            for link in &record.image_links {
                let archive = link.archive.to_ascii_lowercase();
                image_records
                    .entry((archive.clone(), link.group_code, link.icon_id))
                    .or_default()
                    .push(record_index);
                image_group_source_sets
                    .entry((archive, link.group_code))
                    .or_default()
                    .insert(record.source_label.clone());
            }
        }
        let image_group_sources = image_group_source_sets
            .into_iter()
            .filter_map(|(key, sources)| {
                (sources.len() == 1).then(|| (key, sources.into_iter().next().unwrap_or_default()))
            })
            .collect();
        Ok(Self {
            selected_language_block: if available_language_block_count == 1 {
                0
            } else {
                language_block_index
            },
            language_blocks: (0..available_language_block_count)
                .map(|index| TextLanguageBlockSummary {
                    index,
                    label: language_block_label(index),
                    is_default: index == DEFAULT_TEXT_LANGUAGE_BLOCK,
                })
                .collect(),
            sources,
            records,
            image_records,
            image_group_sources,
        })
    }

    pub fn summary(&self) -> TextCatalogSummary {
        TextCatalogSummary {
            selected_language_block: self.selected_language_block,
            language_blocks: self.language_blocks.clone(),
            total_count: self.records.len(),
            sources: self.sources.clone(),
        }
    }

    pub fn page(
        &self,
        source: Option<&str>,
        query: &str,
        offset: usize,
        page_size: usize,
    ) -> Result<TextRecordPage, TextCatalogPageError> {
        if !(1..=TEXT_PAGE_SIZE).contains(&page_size) {
            return Err(TextCatalogPageError::InvalidPageSize {
                requested: page_size,
                maximum: TEXT_PAGE_SIZE,
            });
        }
        let source = source
            .map(str::trim)
            .filter(|source| !source.is_empty())
            .map(str::to_ascii_lowercase);
        if let Some(source) = source.as_deref()
            && !self
                .sources
                .iter()
                .any(|entry| entry.file_name.eq_ignore_ascii_case(source))
        {
            return Err(TextCatalogPageError::SourceNotFound {
                source: source.to_owned(),
            });
        }
        let query = query.trim().to_owned();
        let terms = query
            .split_whitespace()
            .map(|term| term.to_lowercase())
            .collect::<Vec<_>>();
        let matching = self
            .records
            .iter()
            .filter(|record| {
                source
                    .as_deref()
                    .is_none_or(|source| record.source.eq_ignore_ascii_case(source))
                    && text_record_matches(record, &terms)
            })
            .collect::<Vec<_>>();
        let total_count = matching.len();
        if offset > 0 && offset >= total_count {
            return Err(TextCatalogPageError::OffsetOutOfRange {
                offset,
                total_count,
            });
        }
        let end = offset.saturating_add(page_size).min(total_count);
        let items = matching
            .get(offset..end)
            .unwrap_or_default()
            .iter()
            .map(|record| TextRecordItem {
                source: record.source.clone(),
                source_label: record.source_label.clone(),
                id: record.id,
                fields: record.fields.clone(),
                image_links: record.image_links.clone(),
            })
            .collect();

        Ok(TextRecordPage {
            source,
            query,
            offset,
            page_size,
            total_count,
            items,
        })
    }

    pub fn snapshot(&self) -> TextSnapshot {
        TextSnapshot::new(
            self.records
                .iter()
                .map(|record| TextSnapshotEntry {
                    source: record.source.clone(),
                    id: record.id,
                    content_hash: content_hash(&record.fields),
                })
                .collect(),
        )
    }

    pub fn image_relation_snapshot(&self) -> TextImageRelationSnapshot {
        TextImageRelationSnapshot::new(
            self.records
                .iter()
                .flat_map(|record| {
                    record
                        .image_links
                        .iter()
                        .map(|link| TextImageRelationSnapshotEntry {
                            source: record.source.clone(),
                            source_label: record.source_label.clone(),
                            text_id: record.id,
                            archive: link.archive.clone(),
                            group_code: link.group_code,
                            icon_id: link.icon_id,
                            relation: link.relation.clone(),
                            evidence_type: link.evidence_type,
                            verification_status: link.verification_status,
                        })
                })
                .collect(),
        )
    }

    pub fn records_for_keys(&self, keys: &[(String, u32)]) -> Vec<TextRecordItem> {
        let wanted = keys
            .iter()
            .map(|(source, id)| ((source.to_ascii_lowercase(), *id), ()))
            .collect::<BTreeMap<_, _>>();
        self.records
            .iter()
            .filter(|record| wanted.contains_key(&(record.source.to_ascii_lowercase(), record.id)))
            .map(|record| TextRecordItem {
                source: record.source.clone(),
                source_label: record.source_label.clone(),
                id: record.id,
                fields: record.fields.clone(),
                image_links: record.image_links.clone(),
            })
            .collect()
    }

    pub fn links_for_image(
        &self,
        archive: &str,
        group_code: u32,
        icon_id: u32,
    ) -> Vec<TextAssetLink> {
        self.image_records
            .get(&(archive.to_ascii_lowercase(), group_code, icon_id))
            .into_iter()
            .flatten()
            .filter_map(|record_index| self.records.get(*record_index))
            .flat_map(|record| {
                record
                    .image_links
                    .iter()
                    .filter(move |link| {
                        link.archive.eq_ignore_ascii_case(archive)
                            && link.group_code == group_code
                            && link.icon_id == icon_id
                    })
                    .map(move |link| TextAssetLink {
                        source: record.source.clone(),
                        source_label: record.source_label.clone(),
                        id: record.id,
                        name: record
                            .fields
                            .iter()
                            .find(|field| !field.is_empty())
                            .cloned()
                            .unwrap_or_default(),
                        description: record.fields.get(1).cloned().unwrap_or_default(),
                        fields: record.fields.clone(),
                        relation: link.relation.clone(),
                        evidence_type: link.evidence_type,
                        verification_status: link.verification_status,
                    })
            })
            .collect()
    }

    /// Returns the master-table type shared by every linked record in an image group.
    ///
    /// A group with mixed source types intentionally returns `None`; callers must not turn a
    /// partially observed or ambiguous group into a semantic category.
    pub(crate) fn source_label_for_image_group(
        &self,
        archive: &str,
        group_code: u32,
    ) -> Option<&str> {
        self.image_group_sources
            .get(&(archive.to_ascii_lowercase(), group_code))
            .map(String::as_str)
    }

    pub(crate) fn linked_images(&self) -> Vec<TextLinkedImageRecord> {
        self.records
            .iter()
            .flat_map(|record| {
                record
                    .image_links
                    .iter()
                    .cloned()
                    .map(move |image| TextLinkedImageRecord {
                        source: record.source.clone(),
                        source_label: record.source_label.clone(),
                        id: record.id,
                        fields: record.fields.clone(),
                        image,
                    })
            })
            .collect()
    }
}

fn selected_block_index(requested: usize, block_count: usize) -> Result<usize, TextCatalogError> {
    if block_count == 1 {
        return Ok(0);
    }
    if requested >= block_count {
        return Err(TextCatalogError::LanguageBlockOutOfRange {
            requested,
            block_count,
        });
    }
    Ok(requested)
}

fn language_block_label(index: usize) -> String {
    match index {
        0 => "일본어 원문 · 블록 #0".to_owned(),
        DEFAULT_TEXT_LANGUAGE_BLOCK => "한국어 · 블록 #1".to_owned(),
        _ => format!("언어 블록 #{index}"),
    }
}

fn master_value_fields(
    source_key: &str,
    values: &[DtMasterValue],
    trade_good_category_names: &BTreeMap<u32, String>,
    discovery_type_names: &BTreeMap<u32, String>,
) -> Vec<String> {
    let mut raw_index = 0usize;
    values
        .iter()
        .map(|value| match value {
            DtMasterValue::Text(text) => text.clone(),
            DtMasterValue::U8(value) => {
                raw_index += 1;
                format_master_numeric_value_with_reference(
                    source_key,
                    raw_index,
                    "u8",
                    *value as u64,
                    trade_good_category_names,
                    discovery_type_names,
                )
            }
            DtMasterValue::U16(value) => {
                raw_index += 1;
                format_master_numeric_value_with_reference(
                    source_key,
                    raw_index,
                    "u16",
                    *value as u64,
                    trade_good_category_names,
                    discovery_type_names,
                )
            }
            DtMasterValue::U32(value) => {
                raw_index += 1;
                format_master_numeric_value_with_reference(
                    source_key,
                    raw_index,
                    "u32",
                    *value as u64,
                    trade_good_category_names,
                    discovery_type_names,
                )
            }
        })
        .collect()
}

fn format_master_numeric_value_with_reference(
    source_key: &str,
    raw_index: usize,
    storage_type: &str,
    value: u64,
    trade_good_category_names: &BTreeMap<u32, String>,
    discovery_type_names: &BTreeMap<u32, String>,
) -> String {
    let formatted = format_master_numeric_value(source_key, raw_index, storage_type, value);
    if let Ok(reference_id) = u32::try_from(value) {
        let referenced_name = match (source_key, raw_index) {
            ("trade_good", 1) => trade_good_category_names.get(&reference_id),
            ("discovery", 1) => discovery_type_names.get(&reference_id),
            _ => None,
        };
        if let Some(name) = referenced_name {
            return format!("{formatted} · {name}");
        }
    }
    formatted
}

fn format_master_numeric_value(
    source_key: &str,
    raw_index: usize,
    storage_type: &str,
    value: u64,
) -> String {
    if (source_key, raw_index) == ("skill", 1) {
        let meaning = match value {
            0 => Some("모험"),
            1 => Some("교역"),
            2 => Some("전투"),
            _ => None,
        };
        if let Some(meaning) = meaning {
            return format!("스킬 유형 ({storage_type}): {value} · {meaning}");
        }
    }
    if (source_key, raw_index) == ("technique", 4) {
        let meaning = match value {
            1 => Some("파워"),
            2 => Some("퀵"),
            3 => Some("페인트"),
            _ => None,
        };
        if let Some(meaning) = meaning {
            return format!("타입 ({storage_type}): {value} · {meaning}");
        }
    }
    match master_numeric_field_label(source_key, raw_index) {
        Some(label) => format!("{label} ({storage_type}): {value}"),
        None => format!("원시 값 {raw_index} ({storage_type}): {value}"),
    }
}

/// Names below come from the table schemas used by the referenced analysis tool. Fields that the
/// tool itself calls p0/w1/etc. intentionally remain raw instead of receiving a guessed meaning.
fn master_numeric_field_label(source_key: &str, raw_index: usize) -> Option<&'static str> {
    match (source_key, raw_index) {
        ("profession", 1) => Some("직업 계열"),
        ("skill", 1) => Some("스킬 유형"),
        ("skill", 3) => Some("스킬창 가로 위치 후보"),
        ("skill", 5) => Some("스킬창 행 위치"),
        ("skill", 8) => Some("습득 비용"),
        ("skill", 9) => Some("직업 참조"),
        ("ocean", 1) => Some("해역군 참조"),
        ("city", 1) => Some("도시 유형 참조"),
        ("city", 2) => Some("소속 국가 참조"),
        ("city", 3) => Some("문화권 참조"),
        ("suburb", 1) => Some("소속 도시 참조"),
        ("suburb", 2) => Some("문화권 참조"),
        ("equipment", 1) => Some("장비 종류"),
        ("equipment", 2) => Some("공격력"),
        ("equipment", 3) => Some("방어력"),
        ("equipment", 4) => Some("복장예절"),
        ("equipment", 5) => Some("변장도"),
        ("equipment", 6) => Some("내구도"),
        ("equipment", 7) => Some("사거리"),
        ("trade_good", 1) => Some("교역품 유형 참조"),
        ("cannon", 1) => Some("포문 수"),
        ("cannon", 2) => Some("관통력"),
        ("cannon", 3) => Some("위치"),
        ("cannon", 4) => Some("사거리"),
        ("cannon", 5) => Some("포탄속도"),
        ("cannon", 6) => Some("작렬범위"),
        ("cannon", 7) => Some("장전속도"),
        ("cannon", 9) => Some("최대 내구도"),
        ("additional_armor", 1) => Some("장갑"),
        ("additional_armor", 2) => Some("항해 속도 감소"),
        ("additional_armor", 3) => Some("내구도"),
        ("special_equipment", 1) => Some("장비 유형"),
        ("special_equipment", 2) => Some("효과"),
        ("special_equipment", 3) => Some("내구도"),
        ("aux_sail", 1) => Some("가로돛성능"),
        ("aux_sail", 2) => Some("세로돛성능"),
        ("aux_sail", 3) => Some("선회속도"),
        ("aux_sail", 4) => Some("내구도"),
        ("figurehead", 1) => Some("재해 방지"),
        ("figurehead", 2) => Some("피로 감소"),
        ("figurehead", 3) => Some("선원 통제"),
        ("figurehead", 4) => Some("포탄 회피"),
        ("figurehead", 5) => Some("내구도"),
        ("ship", 1) => Some("도형·모델링 참조 후보"),
        ("ship", 2) => Some("모델링 참조 후보 2"),
        ("ship", 4) => Some("길이"),
        ("ship", 5) => Some("너비"),
        ("ship", 6) => Some("높이"),
        ("ship", 7) => Some("선형"),
        ("ship", 8) => Some("선종"),
        ("ship", 9) => Some("보조돛 참조"),
        ("discovery", 1) => Some("발견물 유형 참조"),
        ("technique", 4) => Some("타입"),
        ("technique", 5) => Some("랭크"),
        ("technique", 6) => Some("게이지"),
        ("technique", 8) => Some("범위"),
        ("technique", 9) => Some("사정거리"),
        _ => None,
    }
}

fn text_record_matches(record: &CatalogTextRecord, terms: &[String]) -> bool {
    if terms.is_empty() {
        return true;
    }
    let text = format!(
        "{} {} {} {}",
        record.source,
        record.source_label,
        record.id,
        record.fields.join(" ")
    )
    .to_lowercase();
    terms.iter().all(|term| text.contains(term))
}

fn master_image_links(
    rule: MasterImageRule,
    id: u32,
    image_index: &ImageLinkIndex,
) -> Vec<TextImageLink> {
    master_image_targets(rule, id)
        .into_iter()
        .filter_map(|target| {
            image_index.link(
                &target.archive,
                target.group_code,
                target.icon_id,
                &target.relation,
            )
        })
        .collect()
}

fn master_image_targets(rule: MasterImageRule, id: u32) -> Vec<ExpectedImageLink> {
    let target = |archive: &str, group_code, icon_id, relation: &str| ExpectedImageLink {
        archive: archive.to_owned(),
        group_code,
        icon_id,
        relation: relation.to_owned(),
    };
    match rule {
        MasterImageRule::None => Vec::new(),
        MasterImageRule::Fixed {
            archive,
            group_code,
            relation,
        } => vec![target(archive, group_code, id, relation)],
        MasterImageRule::FixedOffset {
            archive,
            group_code,
            id_offset,
            relation,
        } => id
            .checked_add(id_offset)
            .map(|image_id| target(archive, group_code, image_id, relation))
            .into_iter()
            .collect(),
        MasterImageRule::IdDerivedGroup {
            archive,
            divisor,
            relation,
        } => vec![target(archive, id / divisor, id, relation)],
        MasterImageRule::Discovery => vec![
            target("sc", 11, id, "발견물 목록 이미지"),
            target("sd", 0, id, "발견 시 이미지"),
        ],
        MasterImageRule::LegendDiscovery => vec![
            target("sc", 31, id, "전승 획득 아이콘"),
            target("sd", 29, id, "전승 큰 이미지"),
        ],
    }
}

fn content_hash(fields: &[String]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for field in fields {
        for byte in field.as_bytes().iter().copied().chain([0xff]) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

fn terms_unit_hash(units: &[u16]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in units.iter().flat_map(|unit| unit.to_le_bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

pub fn inspect_text_snapshot(
    game_directory: impl AsRef<Path>,
) -> Result<TextSnapshot, TextCatalogError> {
    TextCatalog::load(game_directory).map(|catalog| catalog.snapshot())
}

pub fn inspect_text_image_relation_snapshot(
    game_directory: impl AsRef<Path>,
) -> Result<TextImageRelationSnapshot, TextCatalogError> {
    TextCatalog::load(game_directory).map(|catalog| catalog.image_relation_snapshot())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextSourceSummary {
    pub file_name: String,
    pub label: String,
    pub description: String,
    pub record_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextLanguageBlockSummary {
    pub index: usize,
    pub label: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextCatalogSummary {
    pub selected_language_block: usize,
    pub language_blocks: Vec<TextLanguageBlockSummary>,
    pub total_count: usize,
    pub sources: Vec<TextSourceSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextRecordItem {
    pub source: String,
    pub source_label: String,
    pub id: u32,
    pub fields: Vec<String>,
    pub image_links: Vec<TextImageLink>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TextImageRelationEvidence {
    #[default]
    MasterTypeRule,
    ExplicitRecordReference,
    VisualReview,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TextImageRelationVerification {
    Candidate,
    #[default]
    HumanVerified,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextImageLink {
    pub archive: String,
    pub group_code: u32,
    pub icon_id: u32,
    pub block_index: u32,
    pub relation: String,
    pub evidence_type: TextImageRelationEvidence,
    pub verification_status: TextImageRelationVerification,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextImageRelationSnapshotEntry {
    pub source: String,
    pub source_label: String,
    pub text_id: u32,
    pub archive: String,
    pub group_code: u32,
    pub icon_id: u32,
    pub relation: String,
    #[serde(default)]
    pub evidence_type: TextImageRelationEvidence,
    #[serde(default)]
    pub verification_status: TextImageRelationVerification,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextImageRelationSnapshot {
    pub format_version: u32,
    pub relations: Vec<TextImageRelationSnapshotEntry>,
}

impl TextImageRelationSnapshot {
    pub fn new(mut relations: Vec<TextImageRelationSnapshotEntry>) -> Self {
        relations.sort();
        relations.dedup();
        Self {
            format_version: TEXT_IMAGE_RELATION_SNAPSHOT_FORMAT_VERSION,
            relations,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextAssetLink {
    pub source: String,
    pub source_label: String,
    pub id: u32,
    pub name: String,
    pub description: String,
    pub fields: Vec<String>,
    pub relation: String,
    pub evidence_type: TextImageRelationEvidence,
    pub verification_status: TextImageRelationVerification,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextLinkedImageRecord {
    pub source: String,
    pub source_label: String,
    pub id: u32,
    pub fields: Vec<String>,
    pub image: TextImageLink,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextRecordPage {
    pub source: Option<String>,
    pub query: String,
    pub offset: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub items: Vec<TextRecordItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextSnapshotEntry {
    pub source: String,
    pub id: u32,
    pub content_hash: u64,
}

impl TextSnapshotEntry {
    fn key(&self) -> (String, u32) {
        (self.source.to_ascii_lowercase(), self.id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextSnapshot {
    pub format_version: u32,
    pub records: Vec<TextSnapshotEntry>,
}

impl TextSnapshot {
    pub fn new(mut records: Vec<TextSnapshotEntry>) -> Self {
        records.sort_by_key(TextSnapshotEntry::key);
        Self {
            format_version: TEXT_SNAPSHOT_FORMAT_VERSION,
            records,
        }
    }

    pub fn compare_to(&self, current: &Self) -> Result<TextSnapshotDiff, TextSnapshotCompareError> {
        if self.format_version != TEXT_SNAPSHOT_FORMAT_VERSION {
            return Err(TextSnapshotCompareError::UnsupportedFormatVersion {
                version: self.format_version,
            });
        }
        if current.format_version != TEXT_SNAPSHOT_FORMAT_VERSION {
            return Err(TextSnapshotCompareError::UnsupportedFormatVersion {
                version: current.format_version,
            });
        }

        let baseline = self
            .records
            .iter()
            .map(|record| (record.key(), record))
            .collect::<BTreeMap<_, _>>();
        let now = current
            .records
            .iter()
            .map(|record| (record.key(), record))
            .collect::<BTreeMap<_, _>>();
        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut changed = Vec::new();
        let mut unchanged_count = 0;

        for (key, record) in &now {
            match baseline.get(key) {
                None => added.push((*record).clone()),
                Some(previous) if previous.content_hash != record.content_hash => {
                    changed.push(TextSnapshotChange {
                        previous: (*previous).clone(),
                        current: (*record).clone(),
                    });
                }
                Some(_) => unchanged_count += 1,
            }
        }
        for (key, record) in baseline {
            if !now.contains_key(&key) {
                removed.push(record.clone());
            }
        }
        Ok(TextSnapshotDiff {
            added,
            removed,
            changed,
            unchanged_count,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSnapshotChange {
    pub previous: TextSnapshotEntry,
    pub current: TextSnapshotEntry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSnapshotDiff {
    pub added: Vec<TextSnapshotEntry>,
    pub removed: Vec<TextSnapshotEntry>,
    pub changed: Vec<TextSnapshotChange>,
    pub unchanged_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextSnapshotCompareError {
    UnsupportedFormatVersion { version: u32 },
}

impl fmt::Display for TextSnapshotCompareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedFormatVersion { version } => {
                write!(
                    formatter,
                    "지원하지 않는 텍스트 기준점 형식입니다: {version}"
                )
            }
        }
    }
}

impl Error for TextSnapshotCompareError {}

#[derive(Debug)]
pub enum TextCatalogError {
    LanguageBlockOutOfRange {
        requested: usize,
        block_count: usize,
    },
    Read {
        path: PathBuf,
        source: io::Error,
    },
    Parse {
        path: PathBuf,
        source: DtTextParseError,
    },
    ParseMaster {
        path: PathBuf,
        source: DtMasterParseError,
    },
    ReadImageIndex {
        path: PathBuf,
        source: io::Error,
    },
    ParseImageIndex {
        path: PathBuf,
        source: IndexParseError,
    },
    NoSupportedSources {
        local_directory: PathBuf,
    },
}

impl fmt::Display for TextCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LanguageBlockOutOfRange {
                requested,
                block_count,
            } => write!(
                formatter,
                "언어 블록 #{requested}를 선택할 수 없습니다: 전체 {block_count}개"
            ),
            Self::Read { path, source } => {
                write!(
                    formatter,
                    "텍스트 파일을 읽지 못했습니다 ({}): {source}",
                    path.display()
                )
            }
            Self::Parse { path, source } => write!(
                formatter,
                "텍스트 파일을 해석하지 못했습니다 ({}): {source}",
                path.display()
            ),
            Self::ParseMaster { path, source } => write!(
                formatter,
                "마스터 텍스트 파일을 해석하지 못했습니다 ({}): {source}",
                path.display()
            ),
            Self::ReadImageIndex { path, source } => write!(
                formatter,
                "텍스트 연결용 이미지 인덱스를 읽지 못했습니다 ({}): {source}",
                path.display()
            ),
            Self::ParseImageIndex { path, source } => write!(
                formatter,
                "텍스트 연결용 이미지 인덱스를 해석하지 못했습니다 ({}): {source}",
                path.display()
            ),
            Self::NoSupportedSources { local_directory } => write!(
                formatter,
                "지원하는 DT 텍스트 파일을 찾지 못했습니다: {}",
                local_directory.display()
            ),
        }
    }
}

impl Error for TextCatalogError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::LanguageBlockOutOfRange { .. } => None,
            Self::Read { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
            Self::ParseMaster { source, .. } => Some(source),
            Self::ReadImageIndex { source, .. } => Some(source),
            Self::ParseImageIndex { source, .. } => Some(source),
            Self::NoSupportedSources { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextCatalogPageError {
    InvalidPageSize { requested: usize, maximum: usize },
    SourceNotFound { source: String },
    OffsetOutOfRange { offset: usize, total_count: usize },
}

impl fmt::Display for TextCatalogPageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPageSize { requested, maximum } => write!(
                formatter,
                "한 번에 불러올 텍스트 수는 1개부터 {maximum}개까지입니다: {requested}"
            ),
            Self::SourceNotFound { source } => {
                write!(formatter, "텍스트 자료 묶음을 찾지 못했습니다: {source}")
            }
            Self::OffsetOutOfRange {
                offset,
                total_count,
            } => write!(
                formatter,
                "텍스트 시작 위치가 자료 범위를 벗어났습니다: {offset}/{total_count}"
            ),
        }
    }
}

impl Error for TextCatalogPageError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn national_event_sources_use_verified_country_names() {
        let labels = TEXT_SOURCES
            .iter()
            .filter(|source| {
                matches!(
                    source.file_name,
                    "dt000101.bin"
                        | "dt000102.bin"
                        | "dt000103.bin"
                        | "dt000104.bin"
                        | "dt000105.bin"
                        | "dt000106.bin"
                )
            })
            .map(|source| source.label)
            .collect::<Vec<_>>();

        assert_eq!(
            labels,
            [
                "에스파냐 국가 이벤트",
                "포르투갈 국가 이벤트",
                "베네치아 국가 이벤트",
                "프랑스 국가 이벤트",
                "네덜란드 국가 이벤트",
                "잉글랜드 국가 이벤트",
            ]
        );
    }

    #[test]
    fn skill_numeric_fields_show_verified_meanings_and_candidate_position() {
        assert_eq!(
            format_master_numeric_value("skill", 1, "u16", 0),
            "스킬 유형 (u16): 0 · 모험"
        );
        assert_eq!(
            format_master_numeric_value("skill", 1, "u16", 1),
            "스킬 유형 (u16): 1 · 교역"
        );
        assert_eq!(
            format_master_numeric_value("skill", 1, "u16", 2),
            "스킬 유형 (u16): 2 · 전투"
        );
        assert_eq!(
            format_master_numeric_value("skill", 3, "u8", 4),
            "스킬창 가로 위치 후보 (u8): 4"
        );
        assert_eq!(
            format_master_numeric_value("skill", 5, "u16", 220),
            "스킬창 행 위치 (u16): 220"
        );
    }

    #[test]
    fn equipment_formality_uses_the_in_game_korean_label() {
        assert_eq!(
            format_master_numeric_value("equipment", 4, "u32", 30),
            "복장예절 (u32): 30"
        );
    }

    #[test]
    fn trade_good_category_reference_includes_the_resolved_name() {
        let categories = BTreeMap::from([(2, "테스트 유형".to_owned())]);
        assert_eq!(
            format_master_numeric_value_with_reference(
                "trade_good",
                1,
                "u16",
                2,
                &categories,
                &BTreeMap::new(),
            ),
            "교역품 유형 참조 (u16): 2 · 테스트 유형"
        );
    }

    #[test]
    fn discovery_type_reference_includes_the_resolved_name() {
        let discovery_types = BTreeMap::from([(3, "역사유물".to_owned())]);
        assert_eq!(
            format_master_numeric_value_with_reference(
                "discovery",
                1,
                "u16",
                3,
                &BTreeMap::new(),
                &discovery_types,
            ),
            "발견물 유형 참조 (u16): 3 · 역사유물"
        );
    }

    #[test]
    fn verified_item_and_technique_field_names_are_used() {
        assert_eq!(
            format_master_numeric_value("cannon", 5, "u32", 12),
            "포탄속도 (u32): 12"
        );
        assert_eq!(
            format_master_numeric_value("aux_sail", 3, "u32", 2),
            "선회속도 (u32): 2"
        );
        assert_eq!(
            format_master_numeric_value("technique", 4, "u8", 3),
            "타입 (u8): 3 · 페인트"
        );
        assert_eq!(
            format_master_numeric_value("technique", 9, "u16", 80),
            "사정거리 (u16): 80"
        );
        assert_eq!(
            format_master_numeric_value("ship", 2, "u16", 42),
            "모델링 참조 후보 2 (u16): 42"
        );
    }

    #[test]
    fn verified_source_labels_use_in_game_terms() {
        let labels = EXTRA_MASTER_SOURCES
            .iter()
            .map(|source| (source.key, source.label))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(labels["historical_person"], "가나돌");
        assert_eq!(labels["pirate_title"], "해적 호칭");
        assert_eq!(labels["beginner_log"], "세일러즈 가이드");
        assert_eq!(labels["mysterious_sea"], "이상한 바다");
        assert_eq!(labels["biography"], "위인의 장");
        assert_eq!(labels["gemstone_intro"], "잠재능력");
    }

    fn entry(source: &str, id: u32, content_hash: u64) -> TextSnapshotEntry {
        TextSnapshotEntry {
            source: source.to_owned(),
            id,
            content_hash,
        }
    }

    #[test]
    fn compares_text_records_by_source_and_id_instead_of_position() {
        let baseline = TextSnapshot::new(vec![
            entry("dt000005.bin", 100, 1),
            entry("dt000005.bin", 200, 2),
            entry("dt000003.bin", 100, 3),
        ]);
        let current = TextSnapshot::new(vec![
            entry("dt000003.bin", 100, 3),
            entry("dt000005.bin", 100, 9),
            entry("dt000005.bin", 300, 4),
        ]);

        let diff = baseline.compare_to(&current).expect("compare snapshots");
        assert_eq!(diff.added, [entry("dt000005.bin", 300, 4)]);
        assert_eq!(diff.removed, [entry("dt000005.bin", 200, 2)]);
        assert_eq!(diff.changed.len(), 1);
        assert_eq!(diff.changed[0].current, entry("dt000005.bin", 100, 9));
        assert_eq!(diff.unchanged_count, 1);
    }

    #[test]
    fn content_hash_preserves_field_boundaries() {
        assert_ne!(
            content_hash(&["ab".to_owned(), "c".to_owned()]),
            content_hash(&["a".to_owned(), "bc".to_owned()])
        );
    }

    #[test]
    fn creates_only_image_links_present_in_the_index() {
        let image_index = ImageLinkIndex {
            records: BTreeMap::from([
                (("sb".to_owned(), 5, 536_928), 6_299),
                (("sc".to_owned(), 11, 42), 1_550),
                (("sc".to_owned(), 31, 7), 5_366),
                (("sc".to_owned(), 35, 2), 5_605),
                (("sd".to_owned(), 29, 7), 10_263),
            ]),
        };

        let item = master_image_links(
            MasterImageRule::IdDerivedGroup {
                archive: "sb",
                divisor: 100_000,
                relation: "아이템 이미지",
            },
            536_928,
            &image_index,
        );
        assert_eq!(item.len(), 1);
        assert_eq!(item[0].block_index, 6_299);
        assert_eq!(
            item[0].evidence_type,
            TextImageRelationEvidence::MasterTypeRule
        );
        assert_eq!(
            item[0].verification_status,
            TextImageRelationVerification::HumanVerified
        );
        assert!(
            master_image_links(
                MasterImageRule::IdDerivedGroup {
                    archive: "sb",
                    divisor: 100_000,
                    relation: "아이템 이미지",
                },
                536_929,
                &image_index,
            )
            .is_empty()
        );

        let discovery = master_image_links(MasterImageRule::Discovery, 42, &image_index);
        assert_eq!(discovery.len(), 1);
        assert_eq!(discovery[0].archive, "sc");
        assert_eq!(discovery[0].relation, "발견물 목록 이미지");

        let legend = master_image_links(MasterImageRule::LegendDiscovery, 7, &image_index);
        assert_eq!(legend.len(), 2);
        assert_eq!(legend[0].relation, "전승 획득 아이콘");
        assert_eq!(legend[1].relation, "전승 큰 이미지");

        let biography = master_image_links(
            MasterImageRule::FixedOffset {
                archive: "sc",
                group_code: 35,
                id_offset: 1,
                relation: "위인의 장 테마 이미지",
            },
            1,
            &image_index,
        );
        assert_eq!(biography.len(), 1);
        assert_eq!(biography[0].icon_id, 2);
    }

    #[test]
    #[ignore = "requires DHO_GAME_DIRECTORY pointing to an installed client"]
    fn matches_verified_legend_pursuit_and_relic_images_in_real_client() {
        let game_directory = PathBuf::from(
            std::env::var_os("DHO_GAME_DIRECTORY")
                .expect("set DHO_GAME_DIRECTORY to the game installation path"),
        );
        let catalog = TextCatalog::load(&game_directory).expect("parse installed text catalog");

        let legends = catalog
            .page(Some("dt000001.bin:legend_discovery"), "", 0, TEXT_PAGE_SIZE)
            .expect("load legends");
        assert_eq!(legends.total_count, 29);
        assert!(
            legends
                .items
                .iter()
                .all(|record| record.image_links.len() == 2)
        );

        let pursuits = catalog
            .page(
                Some("dt000001.bin:pursuit_production"),
                "",
                0,
                TEXT_PAGE_SIZE,
            )
            .expect("load pursuit production");
        assert_eq!(pursuits.total_count, 7);
        assert!(
            pursuits
                .items
                .iter()
                .all(|record| record.image_links.len() == 1)
        );

        let linked_relic = catalog
            .page(
                Some("dt000001.bin:treasure_relic"),
                "구아디아나 강 여행",
                0,
                TEXT_PAGE_SIZE,
            )
            .expect("load linked treasure relic");
        assert_eq!(linked_relic.items[0].id, 101);
        assert_eq!(linked_relic.items[0].image_links.len(), 1);
        assert_eq!(linked_relic.items[0].image_links[0].group_code, 32);

        let unlinked_relic = catalog
            .page(
                Some("dt000001.bin:treasure_relic"),
                "갈겨쓴 메모",
                0,
                TEXT_PAGE_SIZE,
            )
            .expect("load relic without a dedicated image");
        assert_eq!(unlinked_relic.items[0].id, 1);
        assert!(unlinked_relic.items[0].image_links.is_empty());
    }

    #[test]
    #[ignore = "requires DHO_GAME_DIRECTORY pointing to an installed client"]
    fn matches_verified_technique_biography_equipment_and_university_images() {
        let game_directory = PathBuf::from(
            std::env::var_os("DHO_GAME_DIRECTORY")
                .expect("set DHO_GAME_DIRECTORY to the game installation path"),
        );
        let catalog = TextCatalog::load(&game_directory).expect("parse installed text catalog");

        for (source, query, id, archive, group_code, image_id) in [
            ("dt000001.bin:technique", "내려 베기", 1, "sc", 16, 1),
            (
                "dt000001.bin:crew_equipment",
                "세일러 두건",
                2_600_001,
                "sb",
                26,
                2_600_001,
            ),
            (
                "dt000001.bin:ship_decoration",
                "무지의 깃발",
                2_500_001,
                "sb",
                25,
                2_500_001,
            ),
            (
                "dt000001.bin:university_research",
                "기초 연구동",
                1,
                "sc",
                20,
                1,
            ),
        ] {
            let page = catalog
                .page(Some(source), query, 0, TEXT_PAGE_SIZE)
                .expect("load linked semantic record");
            let record = page
                .items
                .iter()
                .find(|record| record.id == id)
                .expect("linked record ID");
            assert_eq!(record.image_links.len(), 1);
            assert_eq!(record.image_links[0].archive, archive);
            assert_eq!(record.image_links[0].group_code, group_code);
            assert_eq!(record.image_links[0].icon_id, image_id);
        }

        for source in ["dt000001.bin:biography", "dt000001.bin:biography_intro"] {
            let page = catalog
                .page(Some(source), "", 0, TEXT_PAGE_SIZE)
                .expect("load biography source");
            assert_eq!(page.total_count, 11);
            assert!(page.items.iter().all(|record| {
                record.image_links.len() == 1
                    && record.image_links[0].icon_id == record.id + 1
                    && record.image_links[0].group_code == 35
            }));
        }

        let biography_links = catalog.links_for_image("sc", 35, 12);
        assert!(biography_links.iter().any(|link| {
            link.source == "dt000001.bin:biography"
                && link.id == 11
                && link.name == "자연을 만나는 사람"
        }));
        assert!(biography_links.iter().any(|link| {
            link.source == "dt000001.bin:biography_intro"
                && link.id == 11
                && link.name == "마리아 지빌라 메리안"
        }));
    }

    #[test]
    #[ignore = "requires DHO_GAME_DIRECTORY pointing to an installed client"]
    fn parses_configured_real_client_text_catalog() {
        let game_directory = PathBuf::from(
            std::env::var_os("DHO_GAME_DIRECTORY")
                .expect("set DHO_GAME_DIRECTORY to the game installation path"),
        );
        let catalog = TextCatalog::load(&game_directory).expect("parse installed text catalog");
        let summary = catalog.summary();
        assert!(summary.total_count > 57_000);
        assert_eq!(summary.selected_language_block, DEFAULT_TEXT_LANGUAGE_BLOCK);
        assert_eq!(summary.language_blocks.len(), 5);
        assert_eq!(summary.language_blocks[1].label, "한국어 · 블록 #1");
        assert_eq!(
            summary
                .sources
                .iter()
                .filter(|source| source.file_name.starts_with("dt000001.bin:"))
                .count(),
            MASTER_SOURCES.len() + EXTRA_MASTER_SOURCES.len() + 1
        );
        for (file_name, record_count) in [
            ("dt000001.bin:command", 487),
            ("dt000001.bin:profession", 95),
            ("dt000001.bin:city", 227),
            ("dt000001.bin:equipment", 8_733),
            ("dt000001.bin:treasure_relic", 909),
            ("dt000001.bin:beginner_tutorial", 53),
            ("dt000001.bin:voyage_log", 53),
            ("dt000001.bin:legacy_information", 195),
            ("dt000001.bin:constellation_intro", 244),
            ("dt000001.bin:ship_decoration", 321),
            ("dt000001.bin:crew_equipment", 214),
            ("dt000000.bin", 6_743),
            ("dt000002.bin", 8_514),
            ("dt000004.bin", 487),
            ("lc000000.bin", 5),
        ] {
            assert!(summary.sources.iter().any(|source| {
                source.file_name == file_name && source.record_count == record_count
            }));
        }
        assert!(
            summary
                .sources
                .iter()
                .any(|source| source.file_name == "dt000005.bin" && source.record_count > 30_000)
        );
        assert!(summary.sources.iter().any(|source| {
            source.file_name == "dt000001.bin:equipment" && source.record_count > 8_000
        }));
        assert!(summary.sources.iter().all(|source| {
            !source.file_name.starts_with("dt000001.bin:") || source.record_count > 0
        }));
        let imperial_events = catalog
            .page(
                Some("dt000001.bin:emperor_event"),
                "신성 로마 황제",
                0,
                TEXT_PAGE_SIZE,
            )
            .expect("search installed emperor event text");
        assert!(imperial_events.total_count > 0);
        let sail_rigs = catalog
            .page(Some("dt000001.bin:sail_rig"), "", 0, TEXT_PAGE_SIZE)
            .expect("read installed sail rig text");
        assert_eq!(sail_rigs.total_count, 43);
        assert!(
            sail_rigs
                .items
                .iter()
                .all(|item| item.image_links.is_empty())
        );
        let skill = catalog
            .page(Some("dt000001.bin:skill"), "돛 조종", 0, TEXT_PAGE_SIZE)
            .expect("search installed skill text");
        let sail_handling = skill
            .items
            .iter()
            .find(|item| item.fields.first().is_some_and(|name| name == "돛 조종"))
            .expect("sail handling skill");
        assert_eq!(sail_handling.image_links.len(), 1);
        assert_eq!(sail_handling.image_links[0].archive, "sa");
        assert_eq!(sail_handling.image_links[0].group_code, 0);
        assert_eq!(sail_handling.image_links[0].icon_id, sail_handling.id);
        let link = &sail_handling.image_links[0];
        let mut session = crate::ViewerSession::default();
        session.set_resource_directory(game_directory.join("0010"));
        let detail = session
            .text_image_detail(&link.archive, link.group_code, link.icon_id, &link.relation)
            .expect("extract installed linked skill image");
        assert_eq!(detail.icon_id, Some(sail_handling.id));
        assert!(
            detail
                .preview_data_url
                .starts_with("data:image/png;base64,")
        );
        let library_page = session
            .category_page(&["스킬".to_owned()], 0, crate::VIEWER_CATEGORY_PAGE_SIZE)
            .expect("read installed linked library page");
        assert!(
            library_page
                .items
                .iter()
                .flat_map(|item| &item.text_links)
                .any(|link| link.name == "돛 조종")
        );
        assert!(
            library_page
                .items
                .iter()
                .flat_map(|item| &item.text_links)
                .filter(|link| link.name == "돛 조종")
                .all(|link| !link.description.is_empty())
        );
        let equipment_search = session
            .search_page("악톤", 0, crate::VIEWER_CATEGORY_PAGE_SIZE)
            .expect("search installed image library by equipment name");
        assert!(equipment_search.total_count >= 1);
        assert_eq!(equipment_search.items[0].path, ["장비", "몸"]);
        assert!(
            equipment_search.items[0]
                .thumbnail
                .text_links
                .iter()
                .any(|link| link.name == "악톤")
        );
        let equipment_link = equipment_search.items[0]
            .thumbnail
            .text_links
            .iter()
            .find(|link| link.name == "악톤")
            .expect("linked equipment detail fields");
        assert!(
            equipment_link
                .fields
                .iter()
                .any(|field| field.starts_with("공격력"))
        );
        let equipment_text = catalog
            .page(Some("dt000001.bin:equipment"), "악톤", 0, TEXT_PAGE_SIZE)
            .expect("search installed equipment attributes");
        let equipment_fields = &equipment_text.items[0].fields;
        for label in ["장비 종류", "공격력", "방어력", "내구도"] {
            assert!(
                equipment_fields
                    .iter()
                    .any(|field| field.starts_with(label)),
                "missing semantic equipment field {label}: {equipment_fields:?}"
            );
        }
        let japanese = TextCatalog::load_language_block(&game_directory, 0)
            .expect("parse installed Japanese language block");
        assert!(
            japanese
                .page(Some("dt000002.bin"), "はい", 0, TEXT_PAGE_SIZE)
                .expect("search Japanese system text")
                .total_count
                > 0
        );
        session
            .set_text_language_block(0)
            .expect("select Japanese library language");
        let japanese_skill = japanese
            .records_for_keys(&[("dt000001.bin:skill".to_owned(), sail_handling.id)])
            .pop()
            .expect("read corresponding Japanese skill");
        let japanese_skill_name = japanese_skill.fields[0].clone();
        let japanese_library = session
            .search_page(&japanese_skill_name, 0, crate::VIEWER_CATEGORY_PAGE_SIZE)
            .expect("search installed Japanese image library");
        assert!(
            japanese_library.items.iter().any(|item| {
                item.thumbnail
                    .text_links
                    .iter()
                    .any(|link| link.name == japanese_skill_name)
            }),
            "Japanese library did not return {japanese_skill_name:?}: total={}, first={:?}",
            japanese_library.total_count,
            japanese_library
                .items
                .first()
                .map(|item| &item.thumbnail.text_links)
        );
        session
            .set_text_language_block(DEFAULT_TEXT_LANGUAGE_BLOCK)
            .expect("restore Korean library language");
        for language_block in 2..=4 {
            let alternate = TextCatalog::load_language_block(&game_directory, language_block)
                .expect("parse installed alternate language block");
            assert!(alternate.summary().total_count > 50_000);
        }
        let description_search = session
            .search_page(
                "의상을 수납할 수 있는 체스트",
                0,
                crate::VIEWER_CATEGORY_PAGE_SIZE,
            )
            .expect("search installed image library by item description");
        assert!(description_search.items.iter().any(|item| {
            item.thumbnail.text_links.iter().any(|link| {
                link.name == "의상용 체스트"
                    && link.description.contains("의상을 수납할 수 있는 체스트")
            })
        }));
        let game_summary = crate::inspect_game_directory(&game_directory)
            .expect("inspect installed data-linked categories");
        for category_name in ["몸", "머리", "다리", "팔", "무기·도구", "장신구"] {
            assert!(
                game_summary.verified_categories.iter().any(|category| {
                    category.path == ["장비", category_name] && category.asset_count > 0
                }),
                "expected installed {category_name} equipment images: {:?}",
                game_summary.verified_categories
            );
        }
        assert!(
            !game_summary
                .verified_categories
                .iter()
                .any(|category| category.path.iter().any(|segment| segment == "방어구"))
        );
        for legacy_path in [
            &["입항허가", "획득 이미지 (128×128)"][..],
            &["전승", "미발견 이미지 (128×128)"][..],
            &["전투"][..],
            &["전투", "UI 이미지"][..],
        ] {
            assert!(
                !game_summary.verified_categories.iter().any(|category| {
                    category
                        .path
                        .iter()
                        .map(String::as_str)
                        .eq(legacy_path.iter().copied())
                }),
                "legacy block-range category leaked into Viewer: {legacy_path:?}"
            );
        }
        assert!(game_summary.verified_categories.iter().any(|category| {
            category
                .path
                .first()
                .is_some_and(|segment| segment == "미분류")
                && category.path.len() >= 2
                && category.asset_count > 0
        }));
        let page = catalog
            .page(Some("dt000005.bin"), "디에고의 소개장", 0, TEXT_PAGE_SIZE)
            .expect("search installed localized event text");
        assert_eq!(page.items[0].fields, ["디에고의 소개장"]);
        let system_page = catalog
            .page(Some("dt000002.bin"), "", 0, TEXT_PAGE_SIZE)
            .expect("read installed system text");
        assert_eq!(system_page.total_count, 8_514);
        assert_eq!(system_page.items[0].fields, ["예"]);
        assert_eq!(system_page.items[1].fields, ["아니오"]);
        let event_recall = catalog
            .page(Some("dt000003.bin"), "", 0, TEXT_PAGE_SIZE)
            .expect("read installed localized event recall");
        assert_eq!(event_recall.items[0].fields, ["여행을 떠남"]);
        let national_event = catalog
            .page(Some("dt000101.bin"), "", 0, TEXT_PAGE_SIZE)
            .expect("read installed localized national event");
        assert_eq!(
            national_event.items[0].fields,
            ["모험가조합 마스터에게 말을 걸어 주십시오."]
        );
    }

    #[test]
    #[ignore = "requires DHO_GAME_DIRECTORY pointing to an installed client"]
    fn parses_additional_master_text_from_a_real_client() {
        let game_directory = PathBuf::from(
            std::env::var_os("DHO_GAME_DIRECTORY")
                .expect("set DHO_GAME_DIRECTORY to the game installation path"),
        );
        let catalog = TextCatalog::load(&game_directory).expect("parse installed text catalog");
        let summary = catalog.summary();
        assert_eq!(summary.total_count, 101_096);
        assert_eq!(
            summary
                .sources
                .iter()
                .filter(|source| source.file_name.starts_with("dt000001.bin:"))
                .count(),
            MASTER_SOURCES.len() + EXTRA_MASTER_SOURCES.len() + 1
        );
        assert!(summary.sources.iter().all(|source| {
            !source.file_name.starts_with("dt000001.bin:") || source.record_count > 0
        }));
        for (source, count) in [
            ("dt000001.bin:treasure_relic", 909),
            ("dt000001.bin:emperor_event", 99),
            ("dt000001.bin:beginner_tutorial", 53),
            ("dt000001.bin:voyage_log", 53),
            ("dt000001.bin:legacy_information", 195),
            ("dt000001.bin:constellation_intro", 244),
            ("dt000001.bin:ship_decoration", 321),
            ("dt000001.bin:crew_equipment", 214),
            ("dt000001.bin:university_navigation", 302),
            ("dt000001.bin:overland_effect", 20),
            ("dt000001.bin:action_emotion", 27),
        ] {
            assert!(
                summary
                    .sources
                    .iter()
                    .any(|entry| { entry.file_name == source && entry.record_count == count })
            );
        }
        let imperial_events = catalog
            .page(
                Some("dt000001.bin:emperor_event"),
                "신성 로마 황제",
                0,
                TEXT_PAGE_SIZE,
            )
            .expect("search installed emperor event text");
        assert!(imperial_events.total_count > 0);
        let actions = catalog
            .page(
                Some("dt000001.bin:action_emotion"),
                "/stand",
                0,
                TEXT_PAGE_SIZE,
            )
            .expect("search installed action command text");
        assert_eq!(actions.items[0].fields[0], "일어선다");
        let legacy_themes = catalog
            .page(Some("dt000001.bin:legacy_theme"), "", 0, TEXT_PAGE_SIZE)
            .expect("load installed legacy themes");
        assert_eq!(legacy_themes.total_count, 13);
        assert!(
            legacy_themes
                .items
                .iter()
                .all(|record| record.image_links.len() == 1)
        );
        let mathematics = legacy_themes
            .items
            .iter()
            .find(|record| record.id == 3)
            .expect("legacy theme ID 3");
        assert_eq!(mathematics.fields[0], "수의 예지 세계의 계산기");
        assert!(mathematics.fields[1].contains("수학"));
        assert_eq!(mathematics.image_links[0].archive, "sc");
        assert_eq!(mathematics.image_links[0].group_code, 33);
        assert_eq!(mathematics.image_links[0].icon_id, 3);
        assert_eq!(mathematics.image_links[0].block_index, 5_586);
        let treasure_themes = catalog
            .page(
                Some("dt000001.bin:treasure_category"),
                "",
                0,
                TEXT_PAGE_SIZE,
            )
            .expect("load installed treasure hunt themes");
        assert_eq!(treasure_themes.total_count, 27);
        assert!(
            treasure_themes
                .items
                .iter()
                .all(|record| record.image_links.len() == 1)
        );
        let shining_hill = treasure_themes
            .items
            .iter()
            .find(|record| record.id == 1)
            .expect("treasure hunt theme ID 1");
        assert_eq!(shining_hill.fields[0], "빛나는 언덕");
        assert!(shining_hill.fields[1].contains("황금향"));
        assert_eq!(shining_hill.image_links[0].archive, "sc");
        assert_eq!(shining_hill.image_links[0].group_code, 26);
        assert_eq!(shining_hill.image_links[0].icon_id, 1);
        assert_eq!(shining_hill.image_links[0].block_index, 5_229);
    }
}
