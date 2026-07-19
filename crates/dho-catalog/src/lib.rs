// SPDX-License-Identifier: MPL-2.0

//! Human-reviewed category metadata kept separate from raw archive facts.

mod assembly;
mod im;
mod is;
mod kp;
mod sa;
mod sb;
mod sc;
mod sd;
mod se;
mod sf;
mod sg;
mod sh;
mod sw;
mod sx;
mod sy;
mod sz;
mod tm;

pub use assembly::{AssemblyPlan, AssemblyRule, LayeredAssemblyRule, TileOrder};
use serde::Serialize;

/// One raw record identity used by the category resolver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogRecordKey<'a> {
    pub archive: &'a str,
    pub group_code: u32,
    pub icon_id: u32,
    pub block_index: u32,
}

/// A display hierarchy such as `장비 > 방어구 > 머리`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct CategoryPath(&'static [&'static str]);

impl CategoryPath {
    pub const fn new(segments: &'static [&'static str]) -> Self {
        Self(segments)
    }

    pub const fn segments(self) -> &'static [&'static str] {
        self.0
    }

    pub fn display_name(self) -> String {
        self.0.join(" > ")
    }
}

/// Where a category hierarchy came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CategorySource {
    InGame,
    Custom,
    Temporary,
}

/// Review state used for both boundaries and meanings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Unknown,
    Candidate,
    HumanVerified,
    Rejected,
}

/// The resolved category for one record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordClassification {
    pub category: Option<CategoryPath>,
    pub category_source: Option<CategorySource>,
    pub boundary_status: VerificationStatus,
    pub meaning_status: VerificationStatus,
}

impl RecordClassification {
    pub const fn unknown() -> Self {
        Self {
            category: None,
            category_source: None,
            boundary_status: VerificationStatus::Unknown,
            meaning_status: VerificationStatus::Unknown,
        }
    }
}

/// A category suggested for an ID that does not currently have a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReservationSuggestion {
    pub category: CategoryPath,
    pub category_source: CategorySource,
    pub status: VerificationStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuleScope {
    ExactBlock(u32),
    ExactId(u32),
    BlockRange {
        start: u32,
        end: u32,
    },
    IdRange {
        start: u32,
        end: u32,
    },
    #[allow(dead_code)] // Reserved for archives with a human-verified group fallback.
    Group(u32),
}

impl RuleScope {
    fn score(self, key: CatalogRecordKey<'_>) -> Option<(u8, u32)> {
        match self {
            Self::ExactBlock(block_index) if block_index == key.block_index => Some((5, u32::MAX)),
            Self::ExactId(icon_id) if icon_id == key.icon_id => Some((4, u32::MAX)),
            Self::BlockRange { start, end }
                if start <= key.block_index && key.block_index <= end =>
            {
                Some((3, u32::MAX - end.saturating_sub(start)))
            }
            Self::IdRange { start, end } if start <= key.icon_id && key.icon_id <= end => {
                Some((2, u32::MAX - end.saturating_sub(start)))
            }
            Self::Group(group_code) if group_code == key.group_code => Some((1, 0)),
            Self::ExactBlock(_)
            | Self::ExactId(_)
            | Self::BlockRange { .. }
            | Self::IdRange { .. }
            | Self::Group(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RecordRule {
    scope: RuleScope,
    category: Option<CategoryPath>,
    category_source: Option<CategorySource>,
    boundary_status: VerificationStatus,
    meaning_status: VerificationStatus,
}

impl RecordRule {
    pub(crate) const fn verified(
        scope: RuleScope,
        category: CategoryPath,
        category_source: CategorySource,
    ) -> Self {
        Self {
            scope,
            category: Some(category),
            category_source: Some(category_source),
            boundary_status: VerificationStatus::HumanVerified,
            meaning_status: VerificationStatus::HumanVerified,
        }
    }

    pub(crate) const fn temporary(scope: RuleScope, category: CategoryPath) -> Self {
        Self {
            scope,
            category: Some(category),
            category_source: Some(CategorySource::Temporary),
            boundary_status: VerificationStatus::HumanVerified,
            meaning_status: VerificationStatus::Unknown,
        }
    }

    pub(crate) const fn explicit_unknown(scope: RuleScope) -> Self {
        Self {
            scope,
            category: None,
            category_source: None,
            boundary_status: VerificationStatus::HumanVerified,
            meaning_status: VerificationStatus::Unknown,
        }
    }

    const fn classification(self) -> RecordClassification {
        RecordClassification {
            category: self.category,
            category_source: self.category_source,
            boundary_status: self.boundary_status,
            meaning_status: self.meaning_status,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReservationRule {
    start: u32,
    end: u32,
    category: CategoryPath,
    category_source: CategorySource,
}

impl ReservationRule {
    pub(crate) const fn new(
        start: u32,
        end: u32,
        category: CategoryPath,
        category_source: CategorySource,
    ) -> Self {
        Self {
            start,
            end,
            category,
            category_source,
        }
    }

    fn matches(self, icon_id: u32) -> bool {
        self.start <= icon_id && icon_id <= self.end
    }

    const fn span(self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    const fn suggestion(self) -> ReservationSuggestion {
        ReservationSuggestion {
            category: self.category,
            category_source: self.category_source,
            status: VerificationStatus::Candidate,
        }
    }
}

struct Catalog<'a> {
    record_rules: &'a [RecordRule],
    reservation_rules: &'a [ReservationRule],
}

impl<'a> Catalog<'a> {
    const fn new(record_rules: &'a [RecordRule], reservation_rules: &'a [ReservationRule]) -> Self {
        Self {
            record_rules,
            reservation_rules,
        }
    }

    fn classify(&self, key: CatalogRecordKey<'_>) -> RecordClassification {
        let mut best = None::<((u8, u32), RecordRule)>;
        for rule in self.record_rules {
            let Some(score) = rule.scope.score(key) else {
                continue;
            };
            if best.is_none_or(|(best_score, _)| score > best_score) {
                best = Some((score, *rule));
            }
        }

        best.map_or_else(RecordClassification::unknown, |(_, rule)| {
            rule.classification()
        })
    }

    fn reservation(&self, icon_id: u32) -> Option<ReservationSuggestion> {
        self.reservation_rules
            .iter()
            .copied()
            .filter(|rule| rule.matches(icon_id))
            .min_by_key(|rule| rule.span())
            .map(ReservationRule::suggestion)
    }
}

/// Resolves a category for an existing raw record.
pub fn classify_record(key: CatalogRecordKey<'_>) -> RecordClassification {
    if key.archive.eq_ignore_ascii_case("im") {
        Catalog::new(im::RECORD_RULES, &[]).classify(key)
    } else if key.archive.eq_ignore_ascii_case("sa") {
        Catalog::new(sa::RECORD_RULES, &[]).classify(key)
    } else if key.archive.eq_ignore_ascii_case("sb") {
        Catalog::new(sb::RECORD_RULES, sb::RESERVATION_RULES).classify(key)
    } else if key.archive.eq_ignore_ascii_case("sc") {
        Catalog::new(sc::RECORD_RULES, &[]).classify(key)
    } else if key.archive.eq_ignore_ascii_case("sd") {
        Catalog::new(sd::RECORD_RULES, &[]).classify(key)
    } else if key.archive.eq_ignore_ascii_case("se") {
        Catalog::new(se::RECORD_RULES, &[]).classify(key)
    } else if key.archive.eq_ignore_ascii_case("sf") {
        Catalog::new(sf::RECORD_RULES, &[]).classify(key)
    } else if key.archive.eq_ignore_ascii_case("sg") {
        Catalog::new(sg::RECORD_RULES, &[]).classify(key)
    } else if key.archive.eq_ignore_ascii_case("sh") {
        Catalog::new(sh::RECORD_RULES, &[]).classify(key)
    } else if key.archive.eq_ignore_ascii_case("sw") {
        Catalog::new(sw::RECORD_RULES, &[]).classify(key)
    } else if key.archive.eq_ignore_ascii_case("sx") {
        Catalog::new(sx::RECORD_RULES, &[]).classify(key)
    } else if key.archive.eq_ignore_ascii_case("sy") {
        Catalog::new(sy::RECORD_RULES, &[]).classify(key)
    } else if key.archive.eq_ignore_ascii_case("sz") {
        Catalog::new(sz::RECORD_RULES, &[]).classify(key)
    } else if key.archive.eq_ignore_ascii_case("tm") {
        Catalog::new(tm::RECORD_RULES, &[]).classify(key)
    } else if key.archive.eq_ignore_ascii_case("is") {
        Catalog::new(is::RECORD_RULES, &[]).classify(key)
    } else if key.archive.eq_ignore_ascii_case("kp") {
        Catalog::new(kp::RECORD_RULES, &[]).classify(key)
    } else {
        RecordClassification::unknown()
    }
}

/// Suggests a candidate category for a currently unused ID slot.
pub fn reservation_candidate(archive: &str, icon_id: u32) -> Option<ReservationSuggestion> {
    archive
        .eq_ignore_ascii_case("sb")
        .then(|| Catalog::new(sb::RECORD_RULES, sb::RESERVATION_RULES).reservation(icon_id))
        .flatten()
}

/// Returns the verified completed-image range and tile position for one physical block.
pub fn assembly_plan(archive: &str, block_index: u32) -> Option<AssemblyPlan> {
    assembly::find_plan(archive, block_index)
}

/// Returns an unverified assembly candidate for Curator review only.
///
/// Candidate plans must never be displayed by the general-user Viewer until
/// a person has reviewed the completed image and promoted the rule.
pub fn assembly_candidate_plan(archive: &str, block_index: u32) -> Option<AssemblyPlan> {
    assembly::find_candidate_plan(archive, block_index)
}

/// Returns a human-verified two-layer raw-image assembly rule for an archive.
pub fn layered_assembly_rule(archive: &str) -> Option<LayeredAssemblyRule> {
    assembly::find_layered_rule(archive)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(group_code: u32, icon_id: u32) -> CatalogRecordKey<'static> {
        CatalogRecordKey {
            archive: "sb",
            group_code,
            icon_id,
            block_index: 0,
        }
    }

    #[test]
    fn classifies_only_the_reviewed_sh_constellation_blocks() {
        for block_index in [0, 42, 87] {
            let classification = classify_record(CatalogRecordKey {
                archive: "sh",
                group_code: 0,
                icon_id: block_index,
                block_index,
            });
            assert_eq!(
                classification.category.map(CategoryPath::segments),
                Some(&["UI 이미지", "별자리 조사", "별자리 선화 (256×256)"][..])
            );
            assert_eq!(
                classification.boundary_status,
                VerificationStatus::HumanVerified
            );
            assert_eq!(
                classification.meaning_status,
                VerificationStatus::HumanVerified
            );
        }

        assert_eq!(
            classify_record(CatalogRecordKey {
                archive: "sh",
                group_code: 0,
                icon_id: 88,
                block_index: 88,
            }),
            RecordClassification::unknown()
        );
    }

    #[test]
    fn classifies_only_the_reviewed_tm_minimap_blocks() {
        for block_index in [0, 198, 212, 248] {
            let classification = classify_record(CatalogRecordKey {
                archive: "tm",
                group_code: 0,
                icon_id: block_index,
                block_index,
            });
            assert_eq!(
                classification.category.map(CategoryPath::segments),
                Some(&["지도", "도시·항구 미니맵 (180×139~141)"][..])
            );
            assert_eq!(
                classification.boundary_status,
                VerificationStatus::HumanVerified
            );
            assert_eq!(
                classification.meaning_status,
                VerificationStatus::HumanVerified
            );
        }

        assert_eq!(
            classify_record(CatalogRecordKey {
                archive: "tm",
                group_code: 0,
                icon_id: 249,
                block_index: 249,
            }),
            RecordClassification::unknown()
        );
    }

    #[test]
    fn classifies_kp_world_map_layers_and_unclassified_overviews() {
        for block_index in [0, 2_047, 2_048, 4_095] {
            let classification = classify_record(CatalogRecordKey {
                archive: "kp",
                group_code: 0,
                icon_id: block_index,
                block_index,
            });
            assert_eq!(
                classification.category.map(CategoryPath::segments),
                Some(&["지도", "세계지도 (3072×1536)"][..])
            );
            assert_eq!(
                classification.meaning_status,
                VerificationStatus::HumanVerified
            );
        }

        for block_index in [4_096, 4_100] {
            let classification = classify_record(CatalogRecordKey {
                archive: "KP",
                group_code: 0,
                icon_id: block_index,
                block_index,
            });
            assert_eq!(
                classification.category.map(CategoryPath::segments),
                Some(&["지도", "미분류 오버뷰 (256×256)"][..])
            );
            assert_eq!(
                classification.category_source,
                Some(CategorySource::Temporary)
            );
        }

        assert_eq!(
            classify_record(CatalogRecordKey {
                archive: "kp",
                group_code: 0,
                icon_id: 4_101,
                block_index: 4_101,
            }),
            RecordClassification::unknown()
        );
    }

    fn sc_key(group_code: u32, icon_id: u32, block_index: u32) -> CatalogRecordKey<'static> {
        CatalogRecordKey {
            archive: "sc",
            group_code,
            icon_id,
            block_index,
        }
    }

    fn sd_key(block_index: u32) -> CatalogRecordKey<'static> {
        CatalogRecordKey {
            archive: "sd",
            group_code: 0,
            icon_id: 0,
            block_index,
        }
    }

    fn is_key(icon_id: u32, block_index: u32) -> CatalogRecordKey<'static> {
        CatalogRecordKey {
            archive: "is",
            group_code: 0,
            icon_id,
            block_index,
        }
    }

    fn im_key(icon_id: u32, block_index: u32) -> CatalogRecordKey<'static> {
        CatalogRecordKey {
            archive: "im",
            group_code: 0,
            icon_id,
            block_index,
        }
    }

    fn sa_key(group_code: u32, icon_id: u32, block_index: u32) -> CatalogRecordKey<'static> {
        CatalogRecordKey {
            archive: "sa",
            group_code,
            icon_id,
            block_index,
        }
    }

    fn se_key(group_code: u32, icon_id: u32, block_index: u32) -> CatalogRecordKey<'static> {
        CatalogRecordKey {
            archive: "se",
            group_code,
            icon_id,
            block_index,
        }
    }

    fn sf_key(group_code: u32, icon_id: u32, block_index: u32) -> CatalogRecordKey<'static> {
        CatalogRecordKey {
            archive: "sf",
            group_code,
            icon_id,
            block_index,
        }
    }

    fn sg_key(group_code: u32, icon_id: u32, block_index: u32) -> CatalogRecordKey<'static> {
        CatalogRecordKey {
            archive: "sg",
            group_code,
            icon_id,
            block_index,
        }
    }

    fn sw_key(group_code: u32, icon_id: u32, block_index: u32) -> CatalogRecordKey<'static> {
        CatalogRecordKey {
            archive: "sw",
            group_code,
            icon_id,
            block_index,
        }
    }

    fn sx_key(group_code: u32, icon_id: u32, block_index: u32) -> CatalogRecordKey<'static> {
        CatalogRecordKey {
            archive: "sx",
            group_code,
            icon_id,
            block_index,
        }
    }

    fn sy_key(group_code: u32, icon_id: u32, block_index: u32) -> CatalogRecordKey<'static> {
        CatalogRecordKey {
            archive: "sy",
            group_code,
            icon_id,
            block_index,
        }
    }

    fn sz_key(group_code: u32, icon_id: u32, block_index: u32) -> CatalogRecordKey<'static> {
        CatalogRecordKey {
            archive: "sz",
            group_code,
            icon_id,
            block_index,
        }
    }

    fn assert_category(icon_id: u32, expected: &[&str]) {
        let classification = classify_record(key(0, icon_id));
        assert_eq!(
            classification.category.map(CategoryPath::segments),
            Some(expected)
        );
        assert_eq!(
            classification.boundary_status,
            VerificationStatus::HumanVerified
        );
        assert_eq!(
            classification.meaning_status,
            VerificationStatus::HumanVerified
        );
    }

    fn assert_sc_category(block_index: u32, expected: &[&str]) {
        let classification = classify_record(sc_key(0, 0, block_index));
        assert_eq!(
            classification.category.map(CategoryPath::segments),
            Some(expected)
        );
        assert_eq!(
            classification.boundary_status,
            VerificationStatus::HumanVerified
        );
        assert_eq!(
            classification.meaning_status,
            VerificationStatus::HumanVerified
        );
    }

    #[test]
    fn classifies_all_is_tiles_as_client_loading_splash_images() {
        for key in [is_key(0, 0), is_key(171, 107)] {
            let classification = classify_record(key);
            assert_eq!(
                classification.category.map(CategoryPath::segments),
                Some(["클라이언트", "로딩·스플래시 이미지"].as_slice())
            );
            assert_eq!(
                classification.boundary_status,
                VerificationStatus::HumanVerified
            );
            assert_eq!(
                classification.meaning_status,
                VerificationStatus::HumanVerified
            );
        }
    }

    #[test]
    fn classifies_the_six_im_images_as_country_selection_maps() {
        for key in [im_key(0, 0), im_key(5, 5)] {
            let classification = classify_record(key);
            assert_eq!(
                classification.category.map(CategoryPath::segments),
                Some(["지도", "국가 선택 지도"].as_slice())
            );
            assert_eq!(
                classification.boundary_status,
                VerificationStatus::HumanVerified
            );
            assert_eq!(
                classification.meaning_status,
                VerificationStatus::HumanVerified
            );
        }
        assert_eq!(
            classify_record(im_key(6, 6)),
            RecordClassification::unknown()
        );
    }

    #[test]
    fn classifies_human_verified_sa_boundaries() {
        for (key, expected) in [
            (sa_key(0, 0, 0), &["인물", "부관 스킬"][..]),
            (sa_key(1, 9_999, 829), &["인물", "부관 스킬"]),
            (sa_key(1, 301, 839), &["UI 아이콘", "감정표현"]),
            (sa_key(2, 372, 910), &["인물", "부관 스킬"]),
            (sa_key(2, 1, 911), &["선박", "선박 그레이드 보너스"]),
            (sa_key(2, 22, 932), &["인물", "부관 스킬"]),
            (sa_key(3, 31, 941), &["선박", "선박 그레이드 보너스"]),
            (sa_key(3, 1, 942), &["UI 아이콘", "연금술"]),
        ] {
            let classification = classify_record(key);
            assert_eq!(
                classification.category.map(CategoryPath::segments),
                Some(expected)
            );
            assert_eq!(
                classification.boundary_status,
                VerificationStatus::HumanVerified
            );
            assert_eq!(
                classification.meaning_status,
                VerificationStatus::HumanVerified
            );
        }

        assert_eq!(
            classify_record(sa_key(3, 99, 944)),
            RecordClassification::unknown()
        );
    }

    #[test]
    fn classifies_all_reviewed_se_labels_as_ui_text() {
        for key in [se_key(0, 0, 0), se_key(0, 1_008, 412)] {
            let classification = classify_record(key);
            assert_eq!(
                classification.category.map(CategoryPath::segments),
                Some(["UI 이미지", "텍스트 라벨"].as_slice())
            );
            assert_eq!(
                classification.boundary_status,
                VerificationStatus::HumanVerified
            );
            assert_eq!(
                classification.meaning_status,
                VerificationStatus::HumanVerified
            );
        }

        assert_eq!(
            classify_record(se_key(1, 0, 0)),
            RecordClassification::unknown()
        );
    }

    #[test]
    fn classifies_reviewed_sf_block_boundaries() {
        for (key, expected) in [
            (sf_key(0, 1, 0), &["UI 이미지", "버튼"][..]),
            (sf_key(1, 4_094, 1_135), &["UI 이미지", "버튼"]),
            (sf_key(1, 1, 1_136), &["UI 아이콘", "원형 아이콘"]),
            (sf_key(1, 275, 1_410), &["UI 아이콘", "원형 아이콘"]),
        ] {
            let classification = classify_record(key);
            assert_eq!(
                classification.category.map(CategoryPath::segments),
                Some(expected)
            );
            assert_eq!(
                classification.boundary_status,
                VerificationStatus::HumanVerified
            );
            assert_eq!(
                classification.meaning_status,
                VerificationStatus::HumanVerified
            );
        }

        assert_eq!(
            classify_record(sf_key(1, 276, 1_411)),
            RecordClassification::unknown()
        );
    }

    #[test]
    fn classifies_reviewed_sg_buttons() {
        for key in [sg_key(0, 1, 0), sg_key(0, 2_125, 1_145)] {
            let classification = classify_record(key);
            assert_eq!(
                classification.category.map(CategoryPath::segments),
                Some(["UI 이미지", "버튼"].as_slice())
            );
            assert_eq!(
                classification.boundary_status,
                VerificationStatus::HumanVerified
            );
            assert_eq!(
                classification.meaning_status,
                VerificationStatus::HumanVerified
            );
        }

        assert_eq!(
            classify_record(sg_key(0, 2_126, 1_146)),
            RecordClassification::unknown()
        );
    }

    #[test]
    fn classifies_reviewed_sw_character_faces() {
        for key in [sw_key(0, 0, 0), sw_key(0, 132, 132)] {
            let classification = classify_record(key);
            assert_eq!(
                classification.category.map(CategoryPath::segments),
                Some(["인물", "캐릭터 얼굴"].as_slice())
            );
            assert_eq!(
                classification.boundary_status,
                VerificationStatus::HumanVerified
            );
            assert_eq!(
                classification.meaning_status,
                VerificationStatus::HumanVerified
            );
        }

        assert_eq!(
            classify_record(sw_key(0, 133, 133)),
            RecordClassification::unknown()
        );
    }

    #[test]
    fn classifies_reviewed_sx_event_portraits() {
        for key in [sx_key(0, 0, 0), sx_key(0, 171, 171), sx_key(0, 219, 219)] {
            let classification = classify_record(key);
            assert_eq!(
                classification.category.map(CategoryPath::segments),
                Some(["이벤트", "포트레잇"].as_slice())
            );
            assert_eq!(
                classification.boundary_status,
                VerificationStatus::HumanVerified
            );
            assert_eq!(
                classification.meaning_status,
                VerificationStatus::HumanVerified
            );
        }

        assert_eq!(
            classify_record(sx_key(0, 220, 220)),
            RecordClassification::unknown()
        );
    }

    #[test]
    fn classifies_reviewed_sy_event_illustrations() {
        for key in [sy_key(0, 0, 0), sy_key(0, 81, 81)] {
            let classification = classify_record(key);
            assert_eq!(
                classification.category.map(CategoryPath::segments),
                Some(["이벤트", "삽화"].as_slice())
            );
            assert_eq!(
                classification.boundary_status,
                VerificationStatus::HumanVerified
            );
            assert_eq!(
                classification.meaning_status,
                VerificationStatus::HumanVerified
            );
        }

        assert_eq!(
            classify_record(sy_key(0, 82, 82)),
            RecordClassification::unknown()
        );
    }

    #[test]
    fn classifies_reviewed_sz_event_portraits() {
        for key in [sz_key(0, 0, 0), sz_key(0, 33, 33)] {
            let classification = classify_record(key);
            assert_eq!(
                classification.category.map(CategoryPath::segments),
                Some(["이벤트", "포트레잇"].as_slice())
            );
            assert_eq!(
                classification.boundary_status,
                VerificationStatus::HumanVerified
            );
            assert_eq!(
                classification.meaning_status,
                VerificationStatus::HumanVerified
            );
        }

        assert_eq!(
            classify_record(sz_key(0, 34, 34)),
            RecordClassification::unknown()
        );
    }

    fn assert_sd_category(block_index: u32, expected: &[&str]) {
        let classification = classify_record(sd_key(block_index));
        assert_eq!(
            classification.category.map(CategoryPath::segments),
            Some(expected)
        );
        assert_eq!(
            classification.boundary_status,
            VerificationStatus::HumanVerified
        );
        assert_eq!(
            classification.meaning_status,
            VerificationStatus::HumanVerified
        );
    }

    #[test]
    fn classifies_representative_verified_sb_ranges() {
        for (icon_id, expected) in [
            (200, &["장비", "방어구", "몸"][..]),
            (100_100, &["장비", "방어구", "머리"]),
            (200_100, &["장비", "방어구", "다리"]),
            (300_100, &["장비", "방어구", "팔"]),
            (400_100, &["장비", "무기"]),
            (500_100, &["장비", "도구"]),
            (600_100, &["선박", "선박 장비", "보조돛"]),
            (700_100, &["선박", "선박 장비", "대포"]),
            (800_100, &["선박", "선박 장비", "추가장갑"]),
            (900_001, &["선박", "선박 장비", "특수장비"]),
            (1_000_100, &["선박", "선박 장비", "선수상"]),
            (1_100_001, &["선박", "선박 장비", "문장"]),
            (1_500_001, &["아이템", "소비품"]),
            (1_700_000, &["아이템", "추천장"]),
            (1_800_000, &["아이템", "레시피"]),
            (1_900_001, &["아이템", "보물지도"]),
            (2_200_000, &["선박", "선박 재료"]),
            (2_300_000, &["아이템", "장식품"]),
            (2_400_000, &["아이템", "소비품", "애완동물"]),
            (2_500_001, &["선박", "선박 데코"]),
            (2_600_001, &["선박", "선원 장비"]),
            (2_602_160, &["선박", "선원 장비"]),
        ] {
            assert_category(icon_id, expected);
        }
    }

    #[test]
    fn preserves_reviewed_unknown_records() {
        for icon_id in [1_200_001, 1_200_101, 2_602_161] {
            let classification = classify_record(key(0, icon_id));
            assert_eq!(classification.category, None);
            assert_eq!(
                classification.boundary_status,
                VerificationStatus::HumanVerified
            );
            assert_eq!(classification.meaning_status, VerificationStatus::Unknown);
        }

        assert_eq!(
            classify_record(key(0, 1_200_002)),
            RecordClassification::unknown()
        );
    }

    #[test]
    fn keeps_temporary_review_bucket_semantically_unknown() {
        let classification = classify_record(key(13, 1_390_010));

        assert_eq!(
            classification.category.map(CategoryPath::segments),
            Some(&["미분류", "SB 특수 아이콘"][..])
        );
        assert_eq!(
            classification.category_source,
            Some(CategorySource::Temporary)
        );
        assert_eq!(
            classification.boundary_status,
            VerificationStatus::HumanVerified
        );
        assert_eq!(classification.meaning_status, VerificationStatus::Unknown);
    }

    #[test]
    fn custom_exact_rules_do_not_claim_unseen_neighbor_ids() {
        let supply = classify_record(key(14, 1_400_100));
        assert_eq!(supply.category_source, Some(CategorySource::Custom));
        assert_eq!(
            supply.category.map(CategoryPath::segments),
            Some(&["선박", "선박 물자"][..])
        );
        assert_eq!(
            classify_record(key(14, 1_400_101)),
            RecordClassification::unknown()
        );
    }

    #[test]
    fn exchange_tickets_share_the_custom_item_acquisition_category() {
        for icon_id in [2_000_000, 2_100_000] {
            let classification = classify_record(key(21, icon_id));
            assert_eq!(
                classification.category.map(CategoryPath::segments),
                Some(&["아이템", "아이템 획득"][..])
            );
            assert_eq!(classification.category_source, Some(CategorySource::Custom));
        }
    }

    #[test]
    fn unsupported_archives_remain_unknown() {
        assert_eq!(
            classify_record(CatalogRecordKey {
                archive: "unknown",
                group_code: 0,
                icon_id: 200,
                block_index: 0,
            }),
            RecordClassification::unknown()
        );
        assert_eq!(reservation_candidate("sd", 200), None);
    }

    #[test]
    fn classifies_representative_verified_sc_block_ranges() {
        for (block_index, expected) in [
            (0, &["교역품"][..]),
            (683, &["선박"]),
            (927, &["선박", "선박 재료", "재질"]),
            (998, &["도시", "도시 내 건물"]),
            (1_028, &["선박", "돛 무늬"]),
            (1_135, &["UI 아이콘", "주점 메뉴"]),
            (1_270, &["UI 아이콘", "카드"]),
            (1_337, &["인물", "부관 아이콘"]),
            (1_473, &["UI 아이콘", "타로"]),
            (1_507, &["발견물", "1", "리스트 이미지 (48×48)"]),
            (4_434, &["아팔타멘토", "집사"]),
            (4_476, &["개인농장"]),
            (4_490, &["UI 아이콘", "생산"]),
            (4_498, &["UI 아이콘", "테크닉"]),
            (4_532, &["개척도시"]),
            (4_656, &["대학·학술협회·부관학교"]),
            (4_921, &["인물", "미분류 초상화"]),
            (5_115, &["기타"]),
            (5_232, &["기타"]),
            (5_233, &["발견물", "2", "리스트 이미지 (48×48)"]),
            (5_265, &["발견물", "2", "리스트 이미지 (48×48)"]),
            (5_266, &["인물", "NPC 초상화"]),
            (5_327, &["인물", "미분류 초상화"]),
            (5_593, &["잠재능력"]),
            (5_940, &["기타"]),
        ] {
            assert_sc_category(block_index, expected);
        }
    }

    #[test]
    fn classifies_verified_sd_block_range_boundaries() {
        for (start, end, expected) in [
            (0, 2_926, &["발견물", "1", "획득 이미지 (128×128)"][..]),
            (3_071, 3_288, &["전투"]),
            (3_289, 3_314, &["입항허가", "획득 이미지 (128×128)"]),
            (3_315, 3_507, &["도움말"]),
            (4_023, 4_026, &["클라이언트", "스플래시 UI"]),
            (7_933, 8_718, &["도움말"]),
            (8_842, 9_248, &["지도", "세계지도"]),
            (9_291, 9_978, &["지도", "필드 지도"]),
            (9_979, 10_011, &["발견물", "2", "획득 이미지 (128×128)"]),
            (10_012, 10_155, &["지도", "던전 지도"]),
            (10_156, 10_175, &["가호"]),
            (10_176, 10_199, &["이벤트"]),
            (10_200, 10_202, &["인물", "미분류 초상화"]),
            (10_203, 10_242, &["클라이언트", "플레이스홀더"]),
            (10_243, 10_271, &["전승", "미발견 이미지 (128×128)"]),
            (10_272, 10_303, &["인물", "구조 부관", "초상화"]),
            (10_304, 10_367, &["인물", "구조 부관", "구조 이미지"]),
            (10_368, 10_395, &["UI 이미지", "예지의 서", "표지"]),
            (
                10_396,
                10_399,
                &["UI 이미지", "예지의 서", "유산의 장", "레거시 테마 UI"],
            ),
            (
                10_400,
                10_418,
                &["UI 이미지", "예지의 서", "유산의 장", "레거시 상세 UI"],
            ),
            (10_439, 10_470, &["UI 이미지", "별자리 조사", "천구도"]),
            (
                10_471,
                10_543,
                &["UI 이미지", "별자리 조사", "별자리선 표시"],
            ),
            (
                10_544,
                10_616,
                &["UI 이미지", "별자리 조사", "별자리선과 그림 표시"],
            ),
            (
                10_617,
                10_800,
                &["UI 이미지", "별자리 조사", "별자리 이미지"],
            ),
            (10_801, 10_811, &["인물", "미분류 초상화"]),
            (10_812, 10_821, &["캐러밴", "낙타"]),
            (10_822, 10_830, &["캐러밴", "대장 초상화"]),
        ] {
            assert_sd_category(start, expected);
            assert_sd_category(end, expected);
        }
    }

    #[test]
    fn sd_unverified_ranges_remain_unclassified() {
        for block_index in [
            2_927, 3_070, 3_508, 4_022, 4_027, 6_268, 7_932, 8_719, 8_841, 9_249, 9_290, 10_419,
            10_438, 10_831,
        ] {
            assert_eq!(
                classify_record(sd_key(block_index)),
                RecordClassification::unknown()
            );
        }
    }

    #[test]
    fn resolves_verified_sd_assembly_rule_boundaries() {
        for (start, end, image_count, columns, rows, width, height) in [
            (10_156, 10_175, 5, 2, 2, 155, 256),
            (10_203, 10_242, 10, 2, 2, 248, 156),
            (10_368, 10_395, 1, 7, 4, 782, 404),
            (10_396, 10_399, 1, 2, 2, 256, 256),
            (10_400, 10_405, 1, 3, 2, 294, 166),
            (10_406, 10_409, 1, 2, 2, 166, 166),
            (10_439, 10_470, 1, 8, 4, 1_024, 512),
            (10_617, 10_800, 46, 2, 2, 192, 192),
        ] {
            let first = assembly_plan("SD", start).expect("first assembly block");
            let last = assembly_plan("sd", end).expect("last assembly block");

            assert_eq!(first.rule.start_block, start);
            assert_eq!(first.rule.end_block, end);
            assert_eq!(first.rule.image_count(), image_count);
            assert_eq!(first.rule.columns, columns);
            assert_eq!(first.rule.rows, rows);
            assert_eq!(first.rule.output_width, width);
            assert_eq!(first.rule.output_height, height);
            assert_eq!(first.rule.status, VerificationStatus::HumanVerified);
            assert_eq!(last.image_index, image_count - 1);
            assert_eq!(last.last_block, end);
            assert_eq!(last.row, rows - 1);
            assert_eq!(last.column, columns - 1);
        }
    }

    #[test]
    fn repeated_sd_assembly_sets_report_image_and_tile_positions() {
        let blessing_last_tile = assembly_plan("sd", 10_159).expect("first blessing end");
        assert_eq!(blessing_last_tile.image_index, 0);
        assert_eq!(blessing_last_tile.first_block, 10_156);
        assert_eq!(blessing_last_tile.last_block, 10_159);
        assert_eq!(blessing_last_tile.tile_index, 3);
        assert_eq!(blessing_last_tile.row, 1);
        assert_eq!(blessing_last_tile.column, 1);

        let next_blessing = assembly_plan("sd", 10_160).expect("second blessing start");
        assert_eq!(next_blessing.image_index, 1);
        assert_eq!(next_blessing.first_block, 10_160);
        assert_eq!(next_blessing.tile_index, 0);
        assert_eq!(next_blessing.row, 0);
        assert_eq!(next_blessing.column, 0);

        for block_index in [8_842, 9_291, 10_012, 10_419, 10_438] {
            assert_eq!(assembly_plan("sd", block_index), None);
        }
        assert_eq!(assembly_plan("sc", 10_156), None);
    }

    #[test]
    fn sc_repeated_ids_are_disambiguated_by_block_index() {
        let preceding_misc = classify_record(sc_key(36, 12, 5_592));
        assert_eq!(
            preceding_misc.category.map(CategoryPath::segments),
            Some(&["기타"][..])
        );

        let potential = classify_record(sc_key(36, 12, 5_604));
        assert_eq!(
            potential.category.map(CategoryPath::segments),
            Some(&["잠재능력"][..])
        );
    }

    #[test]
    fn sc_candidate_ranges_remain_unclassified() {
        for block_index in [4_458, 4_475, 5_311, 5_326] {
            assert_eq!(
                classify_record(sc_key(0, 0, block_index)),
                RecordClassification::unknown()
            );
        }
    }

    #[test]
    fn sc_reviewed_placeholder_is_explicitly_unknown() {
        let classification = classify_record(sc_key(21, 9_999, 4_920));
        assert_eq!(classification.category, None);
        assert_eq!(
            classification.boundary_status,
            VerificationStatus::HumanVerified
        );
        assert_eq!(classification.meaning_status, VerificationStatus::Unknown);
    }

    #[test]
    fn only_verified_reservation_bands_return_candidates() {
        let head = reservation_candidate("SB", 150_000).expect("head reservation");
        assert_eq!(head.category.segments(), &["장비", "방어구", "머리"]);
        assert_eq!(head.status, VerificationStatus::Candidate);

        assert_eq!(reservation_candidate("sb", 1_250_000), None);
        assert_eq!(reservation_candidate("sb", 1_400_101), None);
        assert_eq!(reservation_candidate("sb", 1_650_000), None);
        assert_eq!(reservation_candidate("sb", 2_050_000), None);
        let crew_equipment =
            reservation_candidate("sb", 2_699_999).expect("crew equipment reservation");
        assert_eq!(crew_equipment.category.segments(), &["선박", "선원 장비"]);
        assert_eq!(crew_equipment.status, VerificationStatus::Candidate);

        let reviewed_unknown = classify_record(key(0, 2_602_161));
        assert_eq!(reviewed_unknown.category, None);
        assert_eq!(
            reviewed_unknown.boundary_status,
            VerificationStatus::HumanVerified
        );
    }

    #[test]
    fn exact_block_then_exact_id_then_ranges_then_group_priority_is_stable() {
        const BLOCK_RANGE: CategoryPath = CategoryPath::new(&["블록 범위"]);
        const RANGE: CategoryPath = CategoryPath::new(&["범위"]);
        const EXACT_ID: CategoryPath = CategoryPath::new(&["개별 ID"]);
        const GROUP: CategoryPath = CategoryPath::new(&["그룹"]);
        const RULES: &[RecordRule] = &[
            RecordRule::verified(RuleScope::Group(7), GROUP, CategorySource::InGame),
            RecordRule::verified(
                RuleScope::IdRange { start: 0, end: 100 },
                RANGE,
                CategorySource::InGame,
            ),
            RecordRule::verified(
                RuleScope::BlockRange { start: 10, end: 20 },
                BLOCK_RANGE,
                CategorySource::InGame,
            ),
            RecordRule::verified(RuleScope::ExactId(42), EXACT_ID, CategorySource::InGame),
            RecordRule::explicit_unknown(RuleScope::ExactBlock(15)),
        ];
        let catalog = Catalog::new(RULES, &[]);

        assert_eq!(catalog.classify(sc_key(7, 42, 15)).category, None);
        assert_eq!(
            catalog
                .classify(sc_key(7, 42, 14))
                .category
                .map(CategoryPath::segments),
            Some(&["개별 ID"][..])
        );
        assert_eq!(
            catalog
                .classify(sc_key(7, 50, 14))
                .category
                .map(CategoryPath::segments),
            Some(&["블록 범위"][..])
        );
        assert_eq!(
            catalog
                .classify(sc_key(7, 50, 200))
                .category
                .map(CategoryPath::segments),
            Some(&["범위"][..])
        );
        assert_eq!(
            catalog
                .classify(sc_key(7, 200, 200))
                .category
                .map(CategoryPath::segments),
            Some(&["그룹"][..])
        );
    }
}
