use std::collections::HashSet;

use crate::domain::models::{AppState, BrewingMatchKind, RoastLevelOption};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl ValidationError {
    fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }
}

pub fn validate_app_state(state: &AppState) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    if state.store.name.trim().is_empty() {
        errors.push(ValidationError::new(
            "store.name",
            "store name must not be empty",
        ));
    }
    if let Some(water_tds) = state.store.water_tds
        && water_tds <= 0.0
    {
        errors.push(ValidationError::new(
            "store.water_tds",
            "water_tds must be greater than 0 when provided",
        ));
    }

    let bean_variety_ids = collect_catalog_option_ids(&state.coffee_parameters.bean_varieties);
    validate_catalog_option_labels(
        &state
            .coffee_parameters
            .bean_varieties
            .iter()
            .map(|item| (&item.label, item.archived))
            .collect::<Vec<_>>(),
        "coffee_parameters.bean_varieties",
        &mut errors,
    );
    let processing_method_ids =
        collect_catalog_option_ids(&state.coffee_parameters.processing_methods);
    validate_catalog_option_labels(
        &state
            .coffee_parameters
            .processing_methods
            .iter()
            .map(|item| (&item.label, item.archived))
            .collect::<Vec<_>>(),
        "coffee_parameters.processing_methods",
        &mut errors,
    );
    let roast_level_ids = collect_roast_level_ids(&state.coffee_parameters.roast_levels);
    validate_catalog_option_labels(
        &state
            .coffee_parameters
            .roast_levels
            .iter()
            .map(|item| (&item.label, item.archived))
            .collect::<Vec<_>>(),
        "coffee_parameters.roast_levels",
        &mut errors,
    );

    let bean_ids = collect_active_ids(state.beans.iter().map(|bean| (&bean.id, bean.archived)));
    let roast_method_ids = collect_active_ids(
        state
            .roast_methods
            .iter()
            .map(|method| (&method.id, method.archived)),
    );
    let grinder_ids = collect_active_ids(
        state
            .grinder_profiles
            .iter()
            .map(|grinder| (&grinder.id, grinder.archived)),
    );
    let roast_profile_ids = collect_active_ids(
        state
            .roast_profiles
            .iter()
            .map(|profile| (&profile.id, profile.archived)),
    );

    validate_beans(
        state,
        &bean_variety_ids,
        &processing_method_ids,
        &mut errors,
    );
    validate_roast_methods(state, &mut errors);
    validate_roast_profiles(
        state,
        &bean_ids,
        &roast_method_ids,
        &roast_level_ids,
        &mut errors,
    );
    validate_brewing_plans(
        state,
        &bean_variety_ids,
        &processing_method_ids,
        &roast_level_ids,
        &grinder_ids,
        &mut errors,
    );
    validate_batches(state, &roast_profile_ids, &mut errors);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_beans(
    state: &AppState,
    bean_variety_ids: &HashSet<&str>,
    processing_method_ids: &HashSet<&str>,
    errors: &mut Vec<ValidationError>,
) {
    for bean in &state.beans {
        if bean.name.trim().is_empty() {
            errors.push(ValidationError::new(
                format!("beans[{}].name", bean.id),
                "bean name must not be empty",
            ));
        }
        if let Some(variety_id) = &bean.variety_id
            && !bean_variety_ids.contains(variety_id.as_str())
        {
            errors.push(ValidationError::new(
                format!("beans[{}].variety_id", bean.id),
                format!("referenced variety_id {variety_id} does not exist"),
            ));
        }
        if let Some(processing_method_id) = &bean.processing_method_id
            && !processing_method_ids.contains(processing_method_id.as_str())
        {
            errors.push(ValidationError::new(
                format!("beans[{}].processing_method_id", bean.id),
                format!("referenced processing_method_id {processing_method_id} does not exist"),
            ));
        }
    }
}

fn validate_roast_methods(state: &AppState, errors: &mut Vec<ValidationError>) {
    for method in &state.roast_methods {
        if method.name.trim().is_empty() {
            errors.push(ValidationError::new(
                format!("roast_methods[{}].name", method.id),
                "roast method name must not be empty",
            ));
        }
    }
}

fn validate_roast_profiles(
    state: &AppState,
    bean_ids: &HashSet<&str>,
    roast_method_ids: &HashSet<&str>,
    roast_level_ids: &HashSet<&str>,
    errors: &mut Vec<ValidationError>,
) {
    for profile in &state.roast_profiles {
        if profile.display_name.trim().is_empty() {
            errors.push(ValidationError::new(
                format!("roast_profiles[{}].display_name", profile.id),
                "display name must not be empty",
            ));
        }
        if !bean_ids.contains(profile.bean_id.as_str()) {
            errors.push(ValidationError::new(
                format!("roast_profiles[{}].bean_id", profile.id),
                format!("referenced bean_id {} does not exist", profile.bean_id),
            ));
        }
        if !roast_method_ids.contains(profile.method_id.as_str()) {
            errors.push(ValidationError::new(
                format!("roast_profiles[{}].method_id", profile.id),
                format!("referenced method_id {} does not exist", profile.method_id),
            ));
        }
        if let Some(roast_level_id) = &profile.roast_level_id
            && !roast_level_ids.contains(roast_level_id.as_str())
        {
            errors.push(ValidationError::new(
                format!("roast_profiles[{}].roast_level_id", profile.id),
                format!("referenced roast_level_id {roast_level_id} does not exist"),
            ));
        }
        if profile.batch_code.trim().is_empty() {
            errors.push(ValidationError::new(
                format!("roast_profiles[{}].batch_code", profile.id),
                "batch_code must not be empty",
            ));
        }
    }
}

fn validate_brewing_plans(
    state: &AppState,
    bean_variety_ids: &HashSet<&str>,
    processing_method_ids: &HashSet<&str>,
    roast_level_ids: &HashSet<&str>,
    grinder_ids: &HashSet<&str>,
    errors: &mut Vec<ValidationError>,
) {
    for (category_index, category) in state.brewing_plan_categories.iter().enumerate() {
        if category.name.trim().is_empty() {
            errors.push(ValidationError::new(
                format!("brewing_plan_categories[{category_index}].name"),
                "category name must not be empty",
            ));
        }
        for (plan_index, plan) in category.plans.iter().enumerate() {
            let plan_path =
                format!("brewing_plan_categories[{category_index}].plans[{plan_index}]");
            if plan.name.trim().is_empty() {
                errors.push(ValidationError::new(
                    format!("{plan_path}.name"),
                    "plan name must not be empty",
                ));
            }
            if plan.parameters.dripper.trim().is_empty() {
                errors.push(ValidationError::new(
                    format!("{plan_path}.parameters.dripper"),
                    "dripper must not be empty",
                ));
            }
            if plan.parameters.pour_stages == 0 {
                errors.push(ValidationError::new(
                    format!("{plan_path}.parameters.pour_stages"),
                    "pour_stages must be greater than 0",
                ));
            }
            if plan.parameters.ratio.coffee <= 0.0 || plan.parameters.ratio.water <= 0.0 {
                errors.push(ValidationError::new(
                    format!("{plan_path}.parameters.ratio"),
                    "ratio coffee and water must both be greater than 0",
                ));
            }
            if plan.parameters.default_dose_g <= 0.0 {
                errors.push(ValidationError::new(
                    format!("{plan_path}.parameters.default_dose_g"),
                    "default_dose_g must be greater than 0",
                ));
            }
            if let Some(grinder_profile_id) = &plan.parameters.grinder_profile_id
                && !grinder_ids.contains(grinder_profile_id.as_str())
            {
                errors.push(ValidationError::new(
                    format!("{plan_path}.parameters.grinder_profile_id"),
                    format!("referenced grinder_profile_id {grinder_profile_id} does not exist"),
                ));
            }
            if plan.age_fitting.day0.grind_size <= 0.0
                || plan.age_fitting.day14.grind_size <= 0.0
                || plan.age_fitting.day0.water_temp_c <= 0.0
                || plan.age_fitting.day14.water_temp_c <= 0.0
            {
                errors.push(ValidationError::new(
                    format!("{plan_path}.age_fitting"),
                    "day0/day14 grind and water temperature must be greater than 0",
                ));
            }
            for (attr_index, attr) in plan.matching_attributes.iter().enumerate() {
                let exists = match attr.kind {
                    BrewingMatchKind::BeanVariety => {
                        bean_variety_ids.contains(attr.option_id.as_str())
                    }
                    BrewingMatchKind::ProcessingMethod => {
                        processing_method_ids.contains(attr.option_id.as_str())
                    }
                    BrewingMatchKind::RoastLevel => {
                        roast_level_ids.contains(attr.option_id.as_str())
                    }
                };
                if !exists {
                    errors.push(ValidationError::new(
                        format!("{plan_path}.matching_attributes[{attr_index}].option_id"),
                        format!("referenced option_id {} does not exist", attr.option_id),
                    ));
                }
            }
        }
    }
}

fn validate_batches(
    state: &AppState,
    roast_profile_ids: &HashSet<&str>,
    errors: &mut Vec<ValidationError>,
) {
    for batch in &state.batches {
        if !roast_profile_ids.contains(batch.profile_id.as_str()) {
            errors.push(ValidationError::new(
                format!("batches[{}].profile_id", batch.id),
                format!("referenced profile_id {} does not exist", batch.profile_id),
            ));
        }
        if batch.roasted_at.trim().is_empty() {
            errors.push(ValidationError::new(
                format!("batches[{}].roasted_at", batch.id),
                "roasted_at must not be empty",
            ));
        }
        if batch.batch_no.trim().is_empty() {
            errors.push(ValidationError::new(
                format!("batches[{}].batch_no", batch.id),
                "batch_no must not be empty",
            ));
        }
        if batch.capacity_g <= 0.0 {
            errors.push(ValidationError::new(
                format!("batches[{}].capacity_g", batch.id),
                "capacity_g must be greater than 0",
            ));
        }
    }
}

fn collect_catalog_option_ids(items: &[crate::domain::models::CatalogOption]) -> HashSet<&str> {
    items
        .iter()
        .filter(|item| !item.archived)
        .map(|item| item.id.as_str())
        .collect()
}

fn collect_roast_level_ids(items: &[RoastLevelOption]) -> HashSet<&str> {
    items
        .iter()
        .filter(|item| !item.archived)
        .map(|item| item.id.as_str())
        .collect()
}

fn collect_active_ids<'a>(items: impl Iterator<Item = (&'a String, bool)>) -> HashSet<&'a str> {
    items
        .filter(|(_, archived)| !*archived)
        .map(|(id, _)| id.as_str())
        .collect()
}

fn validate_catalog_option_labels(
    items: &[(&String, bool)],
    field_prefix: &str,
    errors: &mut Vec<ValidationError>,
) {
    let mut seen = HashSet::new();
    for (index, (label, archived)) in items.iter().enumerate() {
        if label.trim().is_empty() {
            errors.push(ValidationError::new(
                format!("{field_prefix}[{index}].label"),
                "catalog label must not be empty",
            ));
        }
        if !archived {
            if !seen.insert(label.as_str()) {
                errors.push(ValidationError::new(
                    format!("{field_prefix}[{index}].label"),
                    "duplicate unarchived label in same catalog",
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::models::{CoffeeBean, ProductLine, RoastMethod, RoastProfile};
    use crate::domain::seed::seed_app_state;

    use super::validate_app_state;

    #[test]
    fn validate_app_state_rejects_empty_name() {
        let mut state = seed_app_state();
        state.store.name.clear();

        let errors = validate_app_state(&state).expect_err("state should be invalid");
        assert_eq!(error_fields(&errors), vec!["store.name"]);
    }

    #[test]
    fn validate_app_state_rejects_empty_batch_code() {
        let mut state = valid_state_with_roast_profile();
        state.roast_profiles[0].batch_code.clear();

        let errors = validate_app_state(&state).expect_err("state should be invalid");
        assert_eq!(
            error_fields(&errors),
            vec!["roast_profiles[profile-1].batch_code"]
        );
    }

    #[test]
    fn validate_app_state_rejects_missing_catalog_reference() {
        let mut state = valid_state_with_roast_profile();
        state.beans[0].processing_method_id = Some("missing-method".to_string());

        let errors = validate_app_state(&state).expect_err("state should be invalid");
        assert_eq!(
            error_fields(&errors),
            vec!["beans[bean-1].processing_method_id"]
        );
    }

    fn error_fields(errors: &[super::ValidationError]) -> Vec<&str> {
        errors.iter().map(|error| error.field.as_str()).collect()
    }

    fn valid_state_with_roast_profile() -> crate::domain::models::AppState {
        let mut state = seed_app_state();
        state.beans.push(CoffeeBean {
            id: "bean-1".to_string(),
            name: "耶加雪菲 G1".to_string(),
            variety_id: Some("bean-var-ethiopian-heirloom".to_string()),
            processing_method_id: Some("process-washed".to_string()),
            origin: Some("Ethiopia".to_string()),
            notes: None,
            archived: false,
        });
        state.roast_methods.push(RoastMethod {
            id: "method-1".to_string(),
            name: "标准手冲曲线".to_string(),
            notes: None,
            archived: false,
        });
        state.roast_profiles.push(RoastProfile {
            id: "profile-1".to_string(),
            bean_id: "bean-1".to_string(),
            method_id: "method-1".to_string(),
            roast_level_id: Some("roast-level-light".to_string()),
            product_line: ProductLine::PourOver,
            display_name: "耶加雪菲浅烘手冲".to_string(),
            batch_code: "YJPO".to_string(),
            recommended_rest_days: Some(7),
            espresso_note: None,
            archived: false,
        });
        state
    }
}
