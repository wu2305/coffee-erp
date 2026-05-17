//! Inventory domain logic for batch creation and recommendation visibility.

use crate::domain::agtron::match_roast_level;
use crate::domain::batch_number::{generate_batch_no, normalize_batch_code};
use crate::domain::models::{AppState, BatchStatus, ProductLine, RoastBatch};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchFormError {
    pub field: String,
    pub message: String,
}

impl BatchFormError {
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }
}

pub fn create_batches(
    state: &mut AppState,
    bean_id: &str,
    product_line: ProductLine,
    roast_level_id: Option<&str>,
    batch_code: &str,
    roasted_at: &str,
    count: u32,
    agtron_score: Option<f32>,
    notes: Option<&str>,
) -> Result<Vec<String>, Vec<BatchFormError>> {
    let mut errors = Vec::new();

    let bean_id = bean_id.trim();
    let batch_code = normalize_batch_code(batch_code);
    let roast_level_id = roast_level_id
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let bean_exists = state
        .beans
        .iter()
        .any(|bean| bean.id == bean_id && !bean.archived);
    if !bean_exists {
        errors.push(BatchFormError::new("bean_id", "请选择有效的咖啡豆"));
    }

    if let Some(level_id) = roast_level_id
        && !state
            .coffee_parameters
            .roast_levels
            .iter()
            .any(|level| level.id == level_id && !level.archived)
    {
        errors.push(BatchFormError::new("roast_level_id", "请选择有效的烘焙度"));
    }

    if batch_code.is_empty() {
        errors.push(BatchFormError::new("batch_code", "请填写批次编码"));
    }

    if count == 0 {
        errors.push(BatchFormError::new("count", "批次数量必须大于 0"));
    }

    let date = parse_roasted_on_date(roasted_at);
    if date.is_none() {
        errors.push(BatchFormError::new(
            "roasted_at",
            "请输入有效的烘焙完成时间",
        ));
    }

    let matched_roast_level_id = if let Some(score) = agtron_score {
        match match_roast_level(score, &state.coffee_parameters.roast_levels) {
            Some(level) => Some(level.id.clone()),
            None => {
                errors.push(BatchFormError::new(
                    "agtron_score",
                    "未找到匹配的烘焙度，请检查参数目录中的 Agtron 范围",
                ));
                None
            }
        }
    } else {
        errors.push(BatchFormError::new(
            "agtron_score",
            "请填写 AG 色值以自动匹配烘焙度",
        ));
        None
    };

    if !errors.is_empty() {
        return Err(errors);
    }

    let date = date.expect("date validated above");
    let notes_owned = notes.map(|value| value.to_string());
    let roast_level_id_owned = matched_roast_level_id.clone();
    let mut created_ids = Vec::new();

    for _ in 0..count {
        let next_id = next_batch_id(&state.batches);
        let batch_no = generate_batch_no(date, &batch_code, &state.batches);
        let batch = RoastBatch {
            id: next_id.clone(),
            profile_id: String::new(),
            bean_id: bean_id.to_string(),
            product_line: Some(product_line),
            roast_level_id: roast_level_id_owned.clone(),
            batch_code: batch_code.clone(),
            roasted_at: roasted_at.to_string(),
            batch_no,
            status: BatchStatus::Active,
            agtron_score,
            matched_roast_level_id: matched_roast_level_id.clone(),
            notes: notes_owned.clone(),
            capacity_g: 100.0,
        };
        created_ids.push(next_id);
        state.batches.push(batch);
    }

    Ok(created_ids)
}

pub fn visible_recommendation_batches(state: &AppState) -> Vec<&RoastBatch> {
    let mut visible: Vec<&RoastBatch> = state
        .batches
        .iter()
        .filter(|batch| batch.status == BatchStatus::Active)
        .collect();
    visible.sort_by(|left, right| {
        left.roasted_at
            .cmp(&right.roasted_at)
            .then(left.batch_no.cmp(&right.batch_no))
    });
    visible
}

fn parse_roasted_on_date(roasted_at: &str) -> Option<chrono::NaiveDate> {
    chrono::DateTime::parse_from_rfc3339(roasted_at)
        .ok()
        .map(|dt| dt.date_naive())
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(roasted_at, "%Y-%m-%dT%H:%M")
                .ok()
                .map(|ndt| ndt.date())
        })
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(roasted_at, "%Y-%m-%dT%H:%M:%S")
                .ok()
                .map(|ndt| ndt.date())
        })
        .or_else(|| chrono::NaiveDate::parse_from_str(roasted_at, "%Y-%m-%d").ok())
}

fn next_batch_id(batches: &[RoastBatch]) -> String {
    let prefix = "batch-";
    let max_suffix = batches
        .iter()
        .filter_map(|batch| batch.id.strip_prefix(prefix))
        .filter_map(|suffix| suffix.parse::<u32>().ok())
        .max()
        .unwrap_or(0);
    format!("{prefix}{:03}", max_suffix + 1)
}

#[cfg(test)]
mod tests {
    use crate::domain::inventory::{
        BatchFormError, create_batches, visible_recommendation_batches,
    };
    use crate::domain::models::{
        BatchStatus, CoffeeBean, ProductLine, RoastBatch, RoastMethod, RoastProfile,
    };
    use crate::domain::seed::seed_app_state;

    fn valid_state_with_profile() -> crate::domain::models::AppState {
        let mut state = seed_app_state();
        state.beans.push(CoffeeBean {
            id: "bean-1".to_string(),
            name: "测试豆".to_string(),
            variety_id: Some("bean-var-bourbon".to_string()),
            processing_method_id: Some("process-washed".to_string()),
            origin: None,
            notes: None,
            archived: false,
        });
        state.roast_methods.push(RoastMethod {
            id: "method-1".to_string(),
            name: "测试曲线".to_string(),
            notes: None,
            archived: false,
        });
        state.roast_profiles.push(RoastProfile {
            id: "profile-1".to_string(),
            bean_id: "bean-1".to_string(),
            method_id: "method-1".to_string(),
            roast_level_id: Some("roast-level-light".to_string()),
            product_line: ProductLine::PourOver,
            display_name: "测试品类".to_string(),
            batch_code: "TEST".to_string(),
            recommended_rest_days: Some(7),
            espresso_note: None,
            archived: false,
        });
        state
    }

    #[test]
    fn create_batches_generates_three_consecutive_batch_numbers() {
        let mut state = valid_state_with_profile();
        let roasted_at = "2026-05-02T08:00:00+00:00";

        let ids = create_batches(
            &mut state,
            "bean-1",
            ProductLine::PourOver,
            Some("roast-level-light"),
            "TEST",
            roasted_at,
            3,
            Some(92.0),
            None,
        )
        .expect("should succeed");

        assert_eq!(ids.len(), 3);
        assert_eq!(state.batches.len(), 3);
        assert_eq!(state.batches[0].batch_no, "20260502-TEST-001");
        assert_eq!(state.batches[1].batch_no, "20260502-TEST-002");
        assert_eq!(state.batches[2].batch_no, "20260502-TEST-003");
        assert_eq!(
            state.batches[0].roast_level_id,
            Some("roast-level-light".to_string())
        );
        assert_eq!(
            state.batches[0].matched_roast_level_id,
            Some("roast-level-light".to_string())
        );
    }

    #[test]
    fn create_batches_rejects_zero_count() {
        let mut state = valid_state_with_profile();

        let errors = create_batches(
            &mut state,
            "bean-1",
            ProductLine::PourOver,
            Some("roast-level-light"),
            "TEST",
            "2026-05-02T08:00:00+00:00",
            0,
            Some(92.0),
            None,
        )
        .expect_err("should reject zero count");

        assert_eq!(
            errors,
            vec![BatchFormError::new("count", "批次数量必须大于 0")]
        );
    }

    #[test]
    fn create_batches_rejects_invalid_bean_id() {
        let mut state = valid_state_with_profile();

        let errors = create_batches(
            &mut state,
            "invalid-bean",
            ProductLine::PourOver,
            Some("roast-level-light"),
            "TEST",
            "2026-05-02T08:00:00+00:00",
            1,
            Some(92.0),
            None,
        )
        .expect_err("should reject invalid bean");

        assert_eq!(
            errors,
            vec![BatchFormError::new("bean_id", "请选择有效的咖啡豆")]
        );
    }

    #[test]
    fn visible_recommendation_batches_includes_active_excludes_used_up_and_archived() {
        let mut state = valid_state_with_profile();
        state.batches.push(RoastBatch {
            id: "batch-active".to_string(),
            profile_id: "profile-1".to_string(),
            bean_id: "bean-1".to_string(),
            product_line: Some(ProductLine::PourOver),
            roast_level_id: Some("roast-level-light".to_string()),
            batch_code: "TEST".to_string(),
            roasted_at: "2026-05-02T08:00:00Z".to_string(),
            batch_no: "20260502-TEST-001".to_string(),
            status: BatchStatus::Active,
            agtron_score: None,
            matched_roast_level_id: None,
            notes: None,
            capacity_g: 100.0,
        });
        state.batches.push(RoastBatch {
            id: "batch-used".to_string(),
            profile_id: "profile-1".to_string(),
            bean_id: "bean-1".to_string(),
            product_line: Some(ProductLine::PourOver),
            roast_level_id: Some("roast-level-light".to_string()),
            batch_code: "TEST".to_string(),
            roasted_at: "2026-05-02T08:00:00Z".to_string(),
            batch_no: "20260502-TEST-002".to_string(),
            status: BatchStatus::UsedUp,
            agtron_score: None,
            matched_roast_level_id: None,
            notes: None,
            capacity_g: 100.0,
        });
        state.batches.push(RoastBatch {
            id: "batch-archived".to_string(),
            profile_id: "profile-1".to_string(),
            bean_id: "bean-1".to_string(),
            product_line: Some(ProductLine::PourOver),
            roast_level_id: Some("roast-level-light".to_string()),
            batch_code: "TEST".to_string(),
            roasted_at: "2026-05-02T08:00:00Z".to_string(),
            batch_no: "20260502-TEST-003".to_string(),
            status: BatchStatus::Archived,
            agtron_score: None,
            matched_roast_level_id: None,
            notes: None,
            capacity_g: 100.0,
        });

        let visible = visible_recommendation_batches(&state);
        let ids: Vec<&str> = visible.iter().map(|batch| batch.id.as_str()).collect();

        assert_eq!(ids, vec!["batch-active"]);
    }

    #[test]
    fn visible_recommendation_batches_shows_three_after_inbound_three() {
        let mut state = valid_state_with_profile();
        create_batches(
            &mut state,
            "bean-1",
            ProductLine::PourOver,
            Some("roast-level-light"),
            "TEST",
            "2026-05-02T08:00:00+00:00",
            3,
            Some(92.0),
            None,
        )
        .expect("should succeed");
        let visible = visible_recommendation_batches(&state);
        assert_eq!(visible.len(), 3);
        let ids: Vec<&str> = visible.iter().map(|b| b.id.as_str()).collect();
        assert_eq!(ids, vec!["batch-001", "batch-002", "batch-003"]);
    }

    #[test]
    fn visible_recommendation_batches_hides_used_up_batch() {
        let mut state = valid_state_with_profile();
        create_batches(
            &mut state,
            "bean-1",
            ProductLine::PourOver,
            Some("roast-level-light"),
            "TEST",
            "2026-05-02T08:00:00+00:00",
            1,
            Some(92.0),
            None,
        )
        .expect("should succeed");
        state.batches[0].status = BatchStatus::UsedUp;
        let visible = visible_recommendation_batches(&state);
        assert!(visible.is_empty());
    }

    #[test]
    fn create_batches_accepts_datetime_local_format() {
        let mut state = valid_state_with_profile();
        let roasted_at = "2026-05-02T08:00";

        let ids = create_batches(
            &mut state,
            "bean-1",
            ProductLine::PourOver,
            Some("roast-level-light"),
            "TEST",
            roasted_at,
            1,
            Some(92.0),
            None,
        )
        .expect("should accept datetime-local format");

        assert_eq!(ids.len(), 1);
        assert_eq!(state.batches[0].batch_no, "20260502-TEST-001");
    }

    #[test]
    fn create_batches_requires_agtron_score_for_roast_level_matching() {
        let mut state = valid_state_with_profile();

        let errors = create_batches(
            &mut state,
            "bean-1",
            ProductLine::PourOver,
            None,
            "TEST",
            "2026-05-02T08:00:00+00:00",
            1,
            None,
            None,
        )
        .expect_err("should require AG score");

        assert_eq!(
            errors,
            vec![BatchFormError::new(
                "agtron_score",
                "请填写 AG 色值以自动匹配烘焙度",
            )]
        );
    }

    #[test]
    fn visible_recommendation_batches_sorts_by_roasted_at_and_batch_no() {
        let mut state = valid_state_with_profile();
        state.batches.push(RoastBatch {
            id: "batch-b".to_string(),
            profile_id: "profile-1".to_string(),
            bean_id: "bean-1".to_string(),
            product_line: Some(ProductLine::PourOver),
            roast_level_id: Some("roast-level-light".to_string()),
            batch_code: "TEST".to_string(),
            roasted_at: "2026-05-03T08:00:00Z".to_string(),
            batch_no: "20260503-TEST-002".to_string(),
            status: BatchStatus::Active,
            agtron_score: None,
            matched_roast_level_id: None,
            notes: None,
            capacity_g: 100.0,
        });
        state.batches.push(RoastBatch {
            id: "batch-a".to_string(),
            profile_id: "profile-1".to_string(),
            bean_id: "bean-1".to_string(),
            product_line: Some(ProductLine::PourOver),
            roast_level_id: Some("roast-level-light".to_string()),
            batch_code: "TEST".to_string(),
            roasted_at: "2026-05-03T08:00:00Z".to_string(),
            batch_no: "20260503-TEST-001".to_string(),
            status: BatchStatus::Active,
            agtron_score: None,
            matched_roast_level_id: None,
            notes: None,
            capacity_g: 100.0,
        });
        state.batches.push(RoastBatch {
            id: "batch-c".to_string(),
            profile_id: "profile-1".to_string(),
            bean_id: "bean-1".to_string(),
            product_line: Some(ProductLine::PourOver),
            roast_level_id: Some("roast-level-light".to_string()),
            batch_code: "TEST".to_string(),
            roasted_at: "2026-05-02T08:00:00Z".to_string(),
            batch_no: "20260502-TEST-001".to_string(),
            status: BatchStatus::Active,
            agtron_score: None,
            matched_roast_level_id: None,
            notes: None,
            capacity_g: 100.0,
        });

        let visible = visible_recommendation_batches(&state);
        let ids: Vec<&str> = visible.iter().map(|batch| batch.id.as_str()).collect();

        assert_eq!(ids, vec!["batch-c", "batch-a", "batch-b"]);
    }
}
