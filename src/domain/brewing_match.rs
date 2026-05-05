use chrono::{DateTime, Utc};

use crate::domain::models::{
    AppState, BrewRatio, BrewingMatchKind, BrewingPlan, ProductLine, RoastBatch,
    WaterQualityAdjustment,
};

#[derive(Debug, Clone, PartialEq)]
pub struct BatchBrewingContext {
    pub bean_variety_id: Option<String>,
    pub processing_method_id: Option<String>,
    pub roast_level_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchedBrewingPlan {
    pub category_id: String,
    pub category_name: String,
    pub category_sort_order: u32,
    pub plan_sort_order: u32,
    pub matched_attribute_count: usize,
    pub plan: BrewingPlan,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FittedBrewingParameters {
    pub grind_size: f32,
    pub water_temp_c: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BrewingRecommendation {
    pub plan_name: String,
    pub dripper: String,
    pub grinder: String,
    pub grind_size: f32,
    pub water_temp_c: f32,
    pub dose_g: f32,
    pub total_water_g: f32,
    pub pour_stages: u8,
}

pub fn resolve_batch_context(batch: &RoastBatch, state: &AppState) -> Option<BatchBrewingContext> {
    let roast_profile = state
        .roast_profiles
        .iter()
        .find(|profile| profile.id == batch.profile_id)?;
    if roast_profile.product_line != ProductLine::PourOver {
        return None;
    }
    let bean = state
        .beans
        .iter()
        .find(|candidate| candidate.id == roast_profile.bean_id)?;
    Some(BatchBrewingContext {
        bean_variety_id: bean.variety_id.clone(),
        processing_method_id: bean.processing_method_id.clone(),
        roast_level_id: roast_profile.roast_level_id.clone(),
    })
}

pub fn match_brewing_plans(batch: &RoastBatch, state: &AppState) -> Vec<MatchedBrewingPlan> {
    let Some(context) = resolve_batch_context(batch, state) else {
        return Vec::new();
    };
    let mut matches = Vec::new();
    for category in &state.brewing_plan_categories {
        if category.archived {
            continue;
        }
        for (plan_index, plan) in category.plans.iter().enumerate() {
            if plan.archived {
                continue;
            }
            let matched = plan.matching_attributes.iter().any(|attr| match attr.kind {
                BrewingMatchKind::BeanVariety => context
                    .bean_variety_id
                    .as_ref()
                    .is_some_and(|id| id == &attr.option_id),
                BrewingMatchKind::ProcessingMethod => context
                    .processing_method_id
                    .as_ref()
                    .is_some_and(|id| id == &attr.option_id),
                BrewingMatchKind::RoastLevel => context
                    .roast_level_id
                    .as_ref()
                    .is_some_and(|id| id == &attr.option_id),
            });
            if matched {
                matches.push(MatchedBrewingPlan {
                    category_id: category.id.clone(),
                    category_name: category.name.clone(),
                    category_sort_order: category.sort_order,
                    plan_sort_order: plan_index as u32,
                    matched_attribute_count: plan.matching_attributes.len(),
                    plan: plan.clone(),
                });
            }
        }
    }
    sort_matched_plans(&mut matches);
    matches
}

pub fn sort_matched_plans(matches: &mut [MatchedBrewingPlan]) {
    matches.sort_by(|left, right| {
        left.matched_attribute_count
            .cmp(&right.matched_attribute_count)
            .then(left.plan.priority.cmp(&right.plan.priority))
            .then(left.category_sort_order.cmp(&right.category_sort_order))
            .then(left.plan_sort_order.cmp(&right.plan_sort_order))
    });
}

pub fn calculate_age_days(roasted_at: DateTime<Utc>, now: DateTime<Utc>) -> f32 {
    let elapsed_hours = now.signed_duration_since(roasted_at).num_seconds() as f64 / 3_600_f64;
    (elapsed_hours / 24_f64) as f32
}

pub fn fit_age_parameters(plan: &BrewingPlan, age_days: f32) -> FittedBrewingParameters {
    let age_ratio = (age_days / 14.0).clamp(0.0, 1.0);
    let day0 = &plan.age_fitting.day0;
    let day14 = &plan.age_fitting.day14;
    FittedBrewingParameters {
        grind_size: day0.grind_size + (day14.grind_size - day0.grind_size) * age_ratio,
        water_temp_c: day0.water_temp_c + (day14.water_temp_c - day0.water_temp_c) * age_ratio,
    }
}

pub fn apply_water_quality_adjustment(
    params: FittedBrewingParameters,
    adjustments: &[WaterQualityAdjustment],
    tds: f32,
) -> FittedBrewingParameters {
    let Some(adjustment) = adjustments.iter().find(|item| tds_in_range(tds, item)) else {
        return params;
    };
    FittedBrewingParameters {
        grind_size: params.grind_size + adjustment.grind_mod,
        water_temp_c: params.water_temp_c + adjustment.temp_mod_c,
    }
}

pub fn calculate_total_water(dose_g: f32, ratio: &BrewRatio) -> f32 {
    dose_g * ratio.water / ratio.coffee
}

pub fn normalize_dose_g(dose_g: f32) -> f32 {
    (dose_g * 10.0).round() / 10.0
}

pub fn parse_roasted_at_utc(roasted_at: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(roasted_at)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

pub fn build_brewing_recommendations(
    batch: &RoastBatch,
    state: &AppState,
    now: DateTime<Utc>,
    dose_g: Option<f32>,
) -> Vec<BrewingRecommendation> {
    let Some(roasted_at) = parse_roasted_at_utc(&batch.roasted_at) else {
        return Vec::new();
    };
    let age_days = calculate_age_days(roasted_at, now);
    let matches = match_brewing_plans(batch, state);
    let selected_dose = normalize_dose_g(dose_g.unwrap_or_else(|| {
        matches
            .first()
            .map(|item| item.plan.parameters.default_dose_g)
            .unwrap_or(0.0)
    }));

    matches
        .into_iter()
        .map(|matched| {
            let fitted = fit_age_parameters(&matched.plan, age_days);
            let adjusted = if let Some(tds) = state.store.water_tds {
                apply_water_quality_adjustment(fitted, &state.water_quality_adjustments, tds)
            } else {
                fitted
            };
            BrewingRecommendation {
                plan_name: matched.plan.name.clone(),
                dripper: matched.plan.parameters.dripper.clone(),
                grinder: resolve_grinder_name(
                    state,
                    matched.plan.parameters.grinder_profile_id.as_deref(),
                ),
                grind_size: adjusted.grind_size,
                water_temp_c: adjusted.water_temp_c,
                dose_g: selected_dose,
                total_water_g: calculate_total_water(selected_dose, &matched.plan.parameters.ratio),
                pour_stages: matched.plan.parameters.pour_stages,
            }
        })
        .collect()
}

fn tds_in_range(tds: f32, adjustment: &WaterQualityAdjustment) -> bool {
    let min_hit = adjustment.tds_min.is_none_or(|min| tds >= min);
    let max_hit = adjustment.tds_max.is_none_or(|max| tds <= max);
    min_hit && max_hit
}

fn resolve_grinder_name(state: &AppState, grinder_profile_id: Option<&str>) -> String {
    let Some(grinder_profile_id) = grinder_profile_id else {
        return String::new();
    };
    state
        .grinder_profiles
        .iter()
        .find(|item| item.id == grinder_profile_id)
        .map(|item| item.name.clone())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone};

    use crate::domain::models::{
        BatchStatus, CoffeeBean, ProductLine, RoastBatch, RoastMethod, RoastProfile,
    };
    use crate::domain::seed::seed_app_state;

    use super::{
        FittedBrewingParameters, MatchedBrewingPlan, apply_water_quality_adjustment,
        build_brewing_recommendations, calculate_age_days, calculate_total_water,
        fit_age_parameters, match_brewing_plans, normalize_dose_g, sort_matched_plans,
    };

    #[test]
    fn match_brewing_plans_washed_hits_washed_plans() {
        let (state, batch) =
            build_state_and_batch("process-washed", "bean-var-bourbon", "roast-level-light");

        let matches = match_brewing_plans(&batch, &state);
        let names: Vec<&str> = matches.iter().map(|item| item.plan.name.as_str()).collect();

        assert_eq!(names, vec!["标准三段式", "锥形滤杯一刀流"]);
    }

    #[test]
    fn match_brewing_plans_sun_dried_hits_strong_sweetness_plan() {
        let (state, batch) =
            build_state_and_batch("process-sun-dried", "bean-var-bourbon", "roast-level-light");

        let matches = match_brewing_plans(&batch, &state);
        let names: Vec<&str> = matches.iter().map(|item| item.plan.name.as_str()).collect();

        assert_eq!(names, vec!["蛋糕滤杯一刀流"]);
    }

    #[test]
    fn match_brewing_plans_dark_roast_hits_volcano_plan() {
        let (state, batch) =
            build_state_and_batch("process-honey", "bean-var-bourbon", "roast-level-dark");

        let matches = match_brewing_plans(&batch, &state);
        let names: Vec<&str> = matches.iter().map(|item| item.plan.name.as_str()).collect();

        assert_eq!(names, vec!["火山冲煮"]);
    }

    #[test]
    fn match_brewing_plans_indonesian_variety_hits_volcano_plan() {
        let (state, batch) =
            build_state_and_batch("process-honey", "bean-var-indonesian", "roast-level-light");

        let matches = match_brewing_plans(&batch, &state);
        let names: Vec<&str> = matches.iter().map(|item| item.plan.name.as_str()).collect();

        assert_eq!(names, vec!["火山冲煮"]);
    }

    #[test]
    fn match_brewing_plans_returns_empty_when_no_match() {
        let (state, batch) =
            build_state_and_batch("process-honey", "bean-var-bourbon", "roast-level-light");

        let matches = match_brewing_plans(&batch, &state);

        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn sort_matched_plans_orders_by_fewer_attributes_first() {
        let mut matches = vec![
            sample_match(3, 1, 1, 1, "plan-a"),
            sample_match(1, 10, 1, 0, "plan-b"),
        ];

        sort_matched_plans(&mut matches);

        assert_eq!(matches[0].plan.id, "plan-b");
        assert_eq!(matches[1].plan.id, "plan-a");
    }

    #[test]
    fn sort_matched_plans_orders_by_priority_when_attribute_count_equal() {
        let mut matches = vec![
            sample_match(2, 2, 1, 0, "plan-a"),
            sample_match(2, 1, 1, 0, "plan-b"),
        ];

        sort_matched_plans(&mut matches);

        assert_eq!(matches[0].plan.id, "plan-b");
        assert_eq!(matches[1].plan.id, "plan-a");
    }

    #[test]
    fn calculate_age_days_counts_24_hours_as_one_day() {
        let roasted_at = chrono::Utc.with_ymd_and_hms(2026, 5, 2, 8, 0, 0).unwrap();
        let now = roasted_at + Duration::hours(24);

        let age_days = calculate_age_days(roasted_at, now);

        assert_approx_eq(age_days, 1.0);
    }

    #[test]
    fn calculate_age_days_counts_12_hours_as_half_day() {
        let roasted_at = chrono::Utc.with_ymd_and_hms(2026, 5, 2, 8, 0, 0).unwrap();
        let now = roasted_at + Duration::hours(12);

        let age_days = calculate_age_days(roasted_at, now);

        assert_approx_eq(age_days, 0.5);
    }

    #[test]
    fn fit_age_parameters_returns_day0_values_on_day0() {
        let state = seed_app_state();
        let plan = find_plan(&state, "plan-cake-one-pour");

        let fitted = fit_age_parameters(plan, 0.0);

        assert_approx_eq(fitted.grind_size, 6.5);
        assert_approx_eq(fitted.water_temp_c, 93.0);
    }

    #[test]
    fn fit_age_parameters_returns_midpoint_values_on_day7() {
        let state = seed_app_state();
        let plan = find_plan(&state, "plan-cake-one-pour");

        let fitted = fit_age_parameters(plan, 7.0);

        assert_approx_eq(fitted.grind_size, 6.75);
        assert_approx_eq(fitted.water_temp_c, 92.0);
    }

    #[test]
    fn fit_age_parameters_returns_day14_values_on_day14() {
        let state = seed_app_state();
        let plan = find_plan(&state, "plan-cake-one-pour");

        let fitted = fit_age_parameters(plan, 14.0);

        assert_approx_eq(fitted.grind_size, 7.0);
        assert_approx_eq(fitted.water_temp_c, 91.0);
    }

    #[test]
    fn fit_age_parameters_caps_at_day14_values_after_day14() {
        let state = seed_app_state();
        let plan = find_plan(&state, "plan-cake-one-pour");

        let fitted = fit_age_parameters(plan, 21.0);

        assert_approx_eq(fitted.grind_size, 7.0);
        assert_approx_eq(fitted.water_temp_c, 91.0);
    }

    #[test]
    fn apply_water_quality_adjustment_assigns_tds_60_to_40_60_range() {
        let state = seed_app_state();
        let params = FittedBrewingParameters {
            grind_size: 7.0,
            water_temp_c: 92.0,
        };

        let adjusted =
            apply_water_quality_adjustment(params, &state.water_quality_adjustments, 60.0);

        assert_approx_eq(adjusted.grind_size, 7.0);
        assert_approx_eq(adjusted.water_temp_c, 92.0);
    }

    #[test]
    fn apply_water_quality_adjustment_assigns_tds_80_to_60_80_range() {
        let state = seed_app_state();
        let params = FittedBrewingParameters {
            grind_size: 7.0,
            water_temp_c: 92.0,
        };

        let adjusted =
            apply_water_quality_adjustment(params, &state.water_quality_adjustments, 80.0);

        assert_approx_eq(adjusted.grind_size, 7.0);
        assert_approx_eq(adjusted.water_temp_c, 91.0);
    }

    #[test]
    fn apply_water_quality_adjustment_assigns_tds_150_to_100_150_range() {
        let state = seed_app_state();
        let params = FittedBrewingParameters {
            grind_size: 7.0,
            water_temp_c: 92.0,
        };

        let adjusted =
            apply_water_quality_adjustment(params, &state.water_quality_adjustments, 150.0);

        assert_approx_eq(adjusted.grind_size, 7.1);
        assert_approx_eq(adjusted.water_temp_c, 91.0);
    }

    #[test]
    fn apply_water_quality_adjustment_assigns_tds_151_to_150_plus_range() {
        let state = seed_app_state();
        let params = FittedBrewingParameters {
            grind_size: 7.0,
            water_temp_c: 92.0,
        };

        let adjusted =
            apply_water_quality_adjustment(params, &state.water_quality_adjustments, 151.0);

        assert_approx_eq(adjusted.grind_size, 7.2);
        assert_approx_eq(adjusted.water_temp_c, 90.0);
    }

    #[test]
    fn calculate_total_water_for_16g_with_ratio_1_16_is_256g() {
        let state = seed_app_state();
        let plan = find_plan(&state, "plan-cone-one-pour");

        let total = calculate_total_water(16.0, &plan.parameters.ratio);

        assert_approx_eq(total, 256.0);
    }

    #[test]
    fn calculate_total_water_for_15_5g_with_ratio_1_15_is_232_5g() {
        let mut state = seed_app_state();
        let plan = state
            .brewing_plan_categories
            .iter_mut()
            .flat_map(|category| category.plans.iter_mut())
            .find(|item| item.id == "plan-cake-one-pour")
            .expect("seed plan exists");
        plan.parameters.ratio.water = 15.0;

        let total = calculate_total_water(15.5, &plan.parameters.ratio);

        assert_approx_eq(total, 232.5);
    }

    #[test]
    fn build_brewing_recommendations_returns_ui_ready_dto() {
        let (mut state, batch) =
            build_state_and_batch("process-sun-dried", "bean-var-bourbon", "roast-level-light");
        state.store.water_tds = Some(80.0);
        let roasted_at = chrono::Utc.with_ymd_and_hms(2026, 5, 2, 8, 0, 0).unwrap();
        let now = roasted_at + Duration::days(7);

        let recommendations = build_brewing_recommendations(&batch, &state, now, Some(15.47));

        assert_eq!(recommendations.len(), 1);
        assert_eq!(recommendations[0].plan_name, "蛋糕滤杯一刀流");
        assert_eq!(recommendations[0].dripper, "马赫");
        assert_eq!(recommendations[0].grinder, "Ditting");
        assert_approx_eq(recommendations[0].grind_size, 6.75);
        assert_approx_eq(recommendations[0].water_temp_c, 91.0);
        assert_approx_eq(recommendations[0].dose_g, normalize_dose_g(15.47));
        assert_approx_eq(recommendations[0].total_water_g, 232.5);
        assert_eq!(recommendations[0].pour_stages, 2);
    }

    fn assert_approx_eq(actual: f32, expected: f32) {
        let tolerance = 1e-6;
        assert!(
            (actual - expected).abs() < tolerance,
            "expected {actual} to be within {tolerance} of {expected}"
        );
    }

    fn find_plan<'a>(
        state: &'a crate::domain::models::AppState,
        plan_id: &str,
    ) -> &'a crate::domain::models::BrewingPlan {
        state
            .brewing_plan_categories
            .iter()
            .flat_map(|category| category.plans.iter())
            .find(|plan| plan.id == plan_id)
            .expect("seed plan exists")
    }

    fn sample_match(
        matched_attribute_count: usize,
        priority: u32,
        category_sort_order: u32,
        plan_sort_order: u32,
        plan_id: &str,
    ) -> MatchedBrewingPlan {
        let state = seed_app_state();
        let mut plan = state.brewing_plan_categories[0].plans[0].clone();
        plan.id = plan_id.to_string();
        plan.priority = priority;
        MatchedBrewingPlan {
            category_id: "category".to_string(),
            category_name: "category".to_string(),
            category_sort_order,
            plan_sort_order,
            matched_attribute_count,
            plan,
        }
    }

    fn build_state_and_batch(
        processing_method_id: &str,
        variety_id: &str,
        roast_level_id: &str,
    ) -> (crate::domain::models::AppState, RoastBatch) {
        let mut state = seed_app_state();
        state.beans.push(CoffeeBean {
            id: "bean-1".to_string(),
            name: "测试豆".to_string(),
            variety_id: Some(variety_id.to_string()),
            processing_method_id: Some(processing_method_id.to_string()),
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
            roast_level_id: Some(roast_level_id.to_string()),
            product_line: ProductLine::PourOver,
            display_name: "测试品类".to_string(),
            batch_code: "TEST".to_string(),
            recommended_rest_days: Some(7),
            espresso_note: None,
            archived: false,
        });
        let batch = RoastBatch {
            id: "batch-1".to_string(),
            profile_id: "profile-1".to_string(),
            roasted_at: "2026-05-02T08:00:00Z".to_string(),
            batch_no: "20260502-TEST-001".to_string(),
            status: BatchStatus::Active,
            notes: None,
        };
        (state, batch)
    }
}
