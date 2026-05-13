use serde::{Deserialize, Serialize};

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Store {
    pub id: String,
    pub name: String,
    pub water_tds: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoffeeParameters {
    pub bean_varieties: Vec<CatalogOption>,
    pub roast_levels: Vec<RoastLevelOption>,
    pub processing_methods: Vec<CatalogOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CatalogOption {
    pub id: String,
    pub label: String,
    pub sort_order: u32,
    pub archived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoastLevelOption {
    pub id: String,
    pub label: String,
    pub agtron_range: String,
    pub agtron_min: Option<f32>,
    pub agtron_max: Option<f32>,
    pub sort_order: u32,
    pub archived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoffeeBean {
    pub id: String,
    pub name: String,
    pub variety_id: Option<String>,
    pub processing_method_id: Option<String>,
    pub origin: Option<String>,
    pub notes: Option<String>,
    pub archived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoastMethod {
    pub id: String,
    pub name: String,
    pub notes: Option<String>,
    pub archived: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProductLine {
    PourOver,
    Espresso,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoastProfile {
    pub id: String,
    pub bean_id: String,
    pub method_id: String,
    pub roast_level_id: Option<String>,
    pub product_line: ProductLine,
    pub display_name: String,
    pub batch_code: String,
    pub recommended_rest_days: Option<u32>,
    pub espresso_note: Option<String>,
    pub archived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GrinderProfile {
    pub id: String,
    pub name: String,
    pub notes: Option<String>,
    pub archived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BrewingPlanCategory {
    pub id: String,
    pub name: String,
    pub sort_order: u32,
    pub plans: Vec<BrewingPlan>,
    pub archived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BrewingPlan {
    pub id: String,
    pub name: String,
    pub matching_attributes: Vec<BrewingMatchAttribute>,
    pub parameters: BrewingPlanParameters,
    pub age_fitting: BrewingAgeFitting,
    pub instructions: Option<String>,
    pub priority: u32,
    pub archived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BrewingMatchAttribute {
    pub kind: BrewingMatchKind,
    pub option_id: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BrewingMatchKind {
    BeanVariety,
    ProcessingMethod,
    RoastLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BrewingPlanParameters {
    pub pour_stages: u8,
    pub dripper: String,
    pub grinder_profile_id: Option<String>,
    pub ratio: BrewRatio,
    pub default_dose_g: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BrewingAgeFitting {
    pub day0: BrewingAgeEndpoint,
    pub day14: BrewingAgeEndpoint,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BrewingAgeEndpoint {
    pub grind_size: f32,
    pub water_temp_c: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WaterQualityAdjustment {
    pub tds_min: Option<f32>,
    pub tds_max: Option<f32>,
    pub temp_mod_c: f32,
    pub grind_mod: f32,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BrewRatio {
    pub coffee: f32,
    pub water: f32,
}

fn default_capacity_g() -> f32 {
    100.0
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoastBatch {
    pub id: String,
    #[serde(default)]
    pub profile_id: String,
    #[serde(default)]
    pub bean_id: String,
    #[serde(default)]
    pub product_line: Option<ProductLine>,
    #[serde(default)]
    pub roast_level_id: Option<String>,
    #[serde(default)]
    pub batch_code: String,
    pub roasted_at: String,
    pub batch_no: String,
    pub status: BatchStatus,
    #[serde(default)]
    pub agtron_score: Option<f32>,
    #[serde(default)]
    pub matched_roast_level_id: Option<String>,
    pub notes: Option<String>,
    #[serde(default = "default_capacity_g")]
    pub capacity_g: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatchStatus {
    Active,
    UsedUp,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppState {
    pub schema_version: u32,
    pub revision: u64,
    pub store: Store,
    pub coffee_parameters: CoffeeParameters,
    pub grinder_profiles: Vec<GrinderProfile>,
    pub water_quality_adjustments: Vec<WaterQualityAdjustment>,
    pub brewing_plan_categories: Vec<BrewingPlanCategory>,
    pub beans: Vec<CoffeeBean>,
    pub roast_methods: Vec<RoastMethod>,
    pub roast_profiles: Vec<RoastProfile>,
    pub batches: Vec<RoastBatch>,
    pub updated_at: String,
}
