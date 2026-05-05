use crate::domain::models::{
    AppState, BrewRatio, BrewingAgeEndpoint, BrewingAgeFitting, BrewingMatchAttribute,
    BrewingMatchKind, BrewingPlan, BrewingPlanCategory, BrewingPlanParameters,
    CURRENT_SCHEMA_VERSION, CatalogOption, CoffeeParameters, GrinderProfile, RoastLevelOption,
    Store, WaterQualityAdjustment,
};

const DEFAULT_UPDATED_AT: &str = "2026-05-02T00:00:00Z";
const DITTING_ID: &str = "grinder-ditting";

pub fn seed_app_state() -> AppState {
    AppState {
        schema_version: CURRENT_SCHEMA_VERSION,
        revision: 0,
        store: Store {
            id: "store-default".to_string(),
            name: "Coffee ERP".to_string(),
            water_tds: None,
        },
        coffee_parameters: CoffeeParameters {
            bean_varieties: seed_bean_varieties(),
            roast_levels: seed_roast_levels(),
            processing_methods: seed_processing_methods(),
        },
        grinder_profiles: vec![GrinderProfile {
            id: DITTING_ID.to_string(),
            name: "Ditting".to_string(),
            notes: None,
            archived: false,
        }],
        water_quality_adjustments: seed_water_quality_adjustments(),
        brewing_plan_categories: seed_brewing_plan_categories(),
        beans: Vec::new(),
        roast_methods: Vec::new(),
        roast_profiles: Vec::new(),
        batches: Vec::new(),
        updated_at: DEFAULT_UPDATED_AT.to_string(),
    }
}

fn seed_bean_varieties() -> Vec<CatalogOption> {
    vec![
        catalog_option("bean-var-geisha", "瑰夏/希爪种", 1),
        catalog_option("bean-var-ethiopian-heirloom", "埃塞原生 (74系)", 2),
        catalog_option("bean-var-bourbon", "波旁", 3),
        catalog_option("bean-var-typica-caturra", "铁皮卡/卡杜拉/", 4),
        catalog_option("bean-var-maragogype", "象豆种", 5),
        catalog_option("bean-var-indonesian", "印尼咖啡", 6),
    ]
}

fn seed_roast_levels() -> Vec<RoastLevelOption> {
    vec![
        roast_level("roast-level-very-light", "极浅", "95+", None, None, 1),
        roast_level(
            "roast-level-light",
            "浅",
            "90-95",
            Some(90.0),
            Some(95.0),
            2,
        ),
        roast_level(
            "roast-level-light-medium",
            "浅中",
            "80-90",
            Some(80.0),
            Some(90.0),
            3,
        ),
        roast_level(
            "roast-level-medium",
            "中",
            "70-80",
            Some(70.0),
            Some(80.0),
            4,
        ),
        roast_level(
            "roast-level-medium-dark",
            "中深",
            "60-70",
            Some(60.0),
            Some(70.0),
            5,
        ),
        roast_level("roast-level-dark", "深", "50-60", Some(50.0), Some(60.0), 6),
    ]
}

fn seed_processing_methods() -> Vec<CatalogOption> {
    vec![
        catalog_option("process-sun-dried", "日晒", 1),
        catalog_option("process-washed", "水洗", 2),
        catalog_option("process-honey", "蜜处理", 3),
        catalog_option("process-light-anaerobic", "轻厌氧", 4),
        catalog_option("process-strong-anaerobic", "强厌氧", 5),
        catalog_option("process-flavor-enhanced", "增味", 6),
    ]
}

fn seed_brewing_plan_categories() -> Vec<BrewingPlanCategory> {
    vec![
        BrewingPlanCategory {
            id: "category-layered-flavor".to_string(),
            name: "强层次，强风味".to_string(),
            sort_order: 1,
            plans: vec![
                BrewingPlan {
                    id: "plan-cone-one-pour".to_string(),
                    name: "锥形滤杯一刀流".to_string(),
                    matching_attributes: vec![
                        match_attr(BrewingMatchKind::ProcessingMethod, "process-washed"),
                        match_attr(
                            BrewingMatchKind::ProcessingMethod,
                            "process-light-anaerobic",
                        ),
                    ],
                    parameters: plan_parameters(2, "RF", Some(DITTING_ID), 1.0, 16.0, 16.0),
                    age_fitting: age_fitting(5.5, 96.0, 6.0, 94.0),
                    instructions: None,
                    priority: 1,
                    archived: false,
                },
                BrewingPlan {
                    id: "plan-standard-three-stage".to_string(),
                    name: "标准三段式".to_string(),
                    matching_attributes: vec![match_attr(
                        BrewingMatchKind::ProcessingMethod,
                        "process-washed",
                    )],
                    parameters: plan_parameters(3, "山文", Some(DITTING_ID), 1.0, 16.0, 16.0),
                    age_fitting: age_fitting(7.1, 93.0, 7.5, 91.0),
                    instructions: None,
                    priority: 2,
                    archived: false,
                },
            ],
            archived: false,
        },
        BrewingPlanCategory {
            id: "category-strong-sweetness".to_string(),
            name: "强甜感".to_string(),
            sort_order: 2,
            plans: vec![BrewingPlan {
                id: "plan-cake-one-pour".to_string(),
                name: "蛋糕滤杯一刀流".to_string(),
                matching_attributes: vec![
                    match_attr(BrewingMatchKind::ProcessingMethod, "process-sun-dried"),
                    match_attr(
                        BrewingMatchKind::ProcessingMethod,
                        "process-light-anaerobic",
                    ),
                    match_attr(
                        BrewingMatchKind::ProcessingMethod,
                        "process-strong-anaerobic",
                    ),
                ],
                parameters: plan_parameters(2, "马赫", Some(DITTING_ID), 1.0, 15.0, 16.0),
                age_fitting: age_fitting(6.5, 93.0, 7.0, 91.0),
                instructions: None,
                priority: 1,
                archived: false,
            }],
            archived: false,
        },
        BrewingPlanCategory {
            id: "category-dark-roast".to_string(),
            name: "深烘".to_string(),
            sort_order: 3,
            plans: vec![BrewingPlan {
                id: "plan-volcano".to_string(),
                name: "火山冲煮".to_string(),
                matching_attributes: vec![
                    match_attr(BrewingMatchKind::RoastLevel, "roast-level-dark"),
                    match_attr(BrewingMatchKind::BeanVariety, "bean-var-indonesian"),
                ],
                parameters: plan_parameters(1, "马赫", Some(DITTING_ID), 1.0, 13.0, 16.0),
                age_fitting: age_fitting(8.5, 85.0, 8.8, 85.0),
                instructions: None,
                priority: 1,
                archived: false,
            }],
            archived: false,
        },
    ]
}

fn seed_water_quality_adjustments() -> Vec<WaterQualityAdjustment> {
    vec![
        WaterQualityAdjustment {
            tds_min: Some(40.0),
            tds_max: Some(60.0),
            temp_mod_c: 0.0,
            grind_mod: 0.0,
            label: "TDS 40-60".to_string(),
        },
        WaterQualityAdjustment {
            tds_min: Some(60.0),
            tds_max: Some(80.0),
            temp_mod_c: -1.0,
            grind_mod: 0.0,
            label: "TDS 60-80".to_string(),
        },
        WaterQualityAdjustment {
            tds_min: Some(80.0),
            tds_max: Some(100.0),
            temp_mod_c: 0.0,
            grind_mod: 0.1,
            label: "TDS 80-100".to_string(),
        },
        WaterQualityAdjustment {
            tds_min: Some(100.0),
            tds_max: Some(150.0),
            temp_mod_c: -1.0,
            grind_mod: 0.1,
            label: "TDS 100-150".to_string(),
        },
        WaterQualityAdjustment {
            tds_min: Some(150.0),
            tds_max: None,
            temp_mod_c: -2.0,
            grind_mod: 0.2,
            label: "TDS 150+".to_string(),
        },
    ]
}

fn catalog_option(id: &str, label: &str, sort_order: u32) -> CatalogOption {
    CatalogOption {
        id: id.to_string(),
        label: label.to_string(),
        sort_order,
        archived: false,
    }
}

fn roast_level(
    id: &str,
    label: &str,
    agtron_range: &str,
    agtron_min: Option<f32>,
    agtron_max: Option<f32>,
    sort_order: u32,
) -> RoastLevelOption {
    RoastLevelOption {
        id: id.to_string(),
        label: label.to_string(),
        agtron_range: agtron_range.to_string(),
        agtron_min,
        agtron_max,
        sort_order,
        archived: false,
    }
}

fn match_attr(kind: BrewingMatchKind, option_id: &str) -> BrewingMatchAttribute {
    BrewingMatchAttribute {
        kind,
        option_id: option_id.to_string(),
    }
}

fn plan_parameters(
    pour_stages: u8,
    dripper: &str,
    grinder_profile_id: Option<&str>,
    coffee: f32,
    water: f32,
    default_dose_g: f32,
) -> BrewingPlanParameters {
    BrewingPlanParameters {
        pour_stages,
        dripper: dripper.to_string(),
        grinder_profile_id: grinder_profile_id.map(str::to_string),
        ratio: BrewRatio { coffee, water },
        default_dose_g,
    }
}

fn age_fitting(
    day0_grind_size: f32,
    day0_water_temp_c: f32,
    day14_grind_size: f32,
    day14_water_temp_c: f32,
) -> BrewingAgeFitting {
    BrewingAgeFitting {
        day0: BrewingAgeEndpoint {
            grind_size: day0_grind_size,
            water_temp_c: day0_water_temp_c,
        },
        day14: BrewingAgeEndpoint {
            grind_size: day14_grind_size,
            water_temp_c: day14_water_temp_c,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::seed_app_state;
    use crate::domain::models::CURRENT_SCHEMA_VERSION;

    #[test]
    fn seed_app_state_contains_expected_catalog_and_plan_counts() {
        let state = seed_app_state();
        assert_eq!(state.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(state.coffee_parameters.bean_varieties.len(), 6);

        let processing_labels: Vec<&str> = state
            .coffee_parameters
            .processing_methods
            .iter()
            .map(|item| item.label.as_str())
            .collect();
        assert_eq!(
            processing_labels,
            vec!["日晒", "水洗", "蜜处理", "轻厌氧", "强厌氧", "增味"]
        );

        assert_eq!(state.grinder_profiles.len(), 1);
        assert_eq!(state.grinder_profiles[0].name, "Ditting");

        assert_eq!(state.brewing_plan_categories.len(), 3);
        let plan_count: usize = state
            .brewing_plan_categories
            .iter()
            .map(|category| category.plans.len())
            .sum();
        assert_eq!(plan_count, 4);
    }

    #[test]
    fn app_state_json_roundtrip_keeps_key_fields() {
        let state = seed_app_state();
        let json = serde_json::to_string(&state).expect("serialize state");
        let deserialized = serde_json::from_str::<crate::domain::models::AppState>(&json)
            .expect("deserialize state");

        assert_eq!(deserialized.schema_version, state.schema_version);
        assert_eq!(deserialized.store.id, state.store.id);
        assert_eq!(
            deserialized.coffee_parameters.processing_methods,
            state.coffee_parameters.processing_methods
        );
        assert_eq!(
            deserialized.brewing_plan_categories.len(),
            state.brewing_plan_categories.len()
        );
    }
}
