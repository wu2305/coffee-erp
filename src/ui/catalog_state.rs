use crate::domain::agtron::roast_level_bounds;
use crate::domain::models::{
    AppState, BrewRatio, BrewingAgeEndpoint, BrewingAgeFitting, BrewingMatchAttribute,
    BrewingMatchKind, BrewingPlan, BrewingPlanCategory, BrewingPlanParameters, CatalogOption,
    CoffeeBean, ProductLine, RoastLevelOption, RoastMethod, RoastProfile,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormValidationError {
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterCatalog {
    BeanVariety,
    ProcessingMethod,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CatalogOptionFormInput {
    pub editing_id: Option<String>,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RoastLevelFormInput {
    pub editing_id: Option<String>,
    pub label: String,
    pub agtron_min: String,
    pub agtron_max: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CoffeeBeanFormInput {
    pub editing_id: Option<String>,
    pub name: String,
    pub variety_id: Option<String>,
    pub processing_method_id: Option<String>,
    pub origin: String,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RoastMethodFormInput {
    pub editing_id: Option<String>,
    pub name: String,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoastProfileFormInput {
    pub editing_id: Option<String>,
    pub bean_id: String,
    pub method_id: String,
    pub roast_level_id: Option<String>,
    pub product_line: ProductLine,
    pub batch_code: String,
}

impl Default for RoastProfileFormInput {
    fn default() -> Self {
        Self {
            editing_id: None,
            bean_id: String::new(),
            method_id: String::new(),
            roast_level_id: None,
            product_line: ProductLine::PourOver,
            batch_code: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BrewingPlanCategoryFormInput {
    pub editing_id: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BrewingPlanFormInput {
    pub editing_id: Option<String>,
    pub category_id: String,
    pub name: String,
    pub matching_attributes: Vec<BrewingMatchAttribute>,
    pub pour_stages: u8,
    pub dripper: String,
    pub grinder_profile_id: Option<String>,
    pub ratio_coffee: f32,
    pub ratio_water: f32,
    pub default_dose_g: f32,
    pub day0_grind_size: f32,
    pub day0_water_temp_c: f32,
    pub day14_grind_size: f32,
    pub day14_water_temp_c: f32,
    pub instructions: String,
    pub priority: u32,
}

impl Default for BrewingPlanFormInput {
    fn default() -> Self {
        Self {
            editing_id: None,
            category_id: String::new(),
            name: String::new(),
            matching_attributes: Vec::new(),
            pour_stages: 3,
            dripper: String::new(),
            grinder_profile_id: None,
            ratio_coffee: 1.0,
            ratio_water: 15.0,
            default_dose_g: 16.0,
            day0_grind_size: 6.0,
            day0_water_temp_c: 92.0,
            day14_grind_size: 7.0,
            day14_water_temp_c: 90.0,
            instructions: String::new(),
            priority: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchiveTarget {
    BeanVariety { id: String },
    RoastLevel { id: String },
    ProcessingMethod { id: String },
    CoffeeBean { id: String },
    RoastMethod { id: String },
    RoastProfile { id: String },
    BrewingPlanCategory { id: String },
    BrewingPlan { category_id: String, id: String },
    BatchUsedUp { id: String },
    BatchArchived { id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingArchive {
    pub target: ArchiveTarget,
    pub remaining_seconds: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingArchiveError {
    AlreadyPending,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchiveCommitError {
    TargetNotFound,
}

pub const DEFAULT_ROAST_METHOD_ID: &str = "roast-method-default";
pub const DEFAULT_ROAST_METHOD_NAME: &str = "标准曲线";

pub fn suggest_batch_code(bean_name: &str, method_name: &str, product_line: ProductLine) -> String {
    let bean_prefix = alnum_prefix(bean_name, 2);
    let method_prefix = alnum_prefix(method_name, 2);
    let line_suffix = match product_line {
        ProductLine::PourOver => "PO",
        ProductLine::Espresso => "ES",
    };
    format!("{bean_prefix}{method_prefix}{line_suffix}")
}

pub fn upsert_parameter_option(
    state: &mut AppState,
    catalog: ParameterCatalog,
    input: &CatalogOptionFormInput,
) -> Result<String, Vec<FormValidationError>> {
    let mut errors = Vec::new();
    let label = input.label.trim();
    if label.is_empty() {
        errors.push(form_error("label", "标签不能为空"));
    }

    let id_to_ignore = input.editing_id.as_deref();
    let has_duplicate = match catalog {
        ParameterCatalog::BeanVariety => has_duplicate_catalog_label(
            &state.coffee_parameters.bean_varieties,
            label,
            id_to_ignore,
        ),
        ParameterCatalog::ProcessingMethod => has_duplicate_catalog_label(
            &state.coffee_parameters.processing_methods,
            label,
            id_to_ignore,
        ),
    };
    if !label.is_empty() && has_duplicate {
        errors.push(form_error("label", "未归档标签不能重复"));
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let id = match catalog {
        ParameterCatalog::BeanVariety => upsert_catalog_option(
            &mut state.coffee_parameters.bean_varieties,
            "bean-variety",
            input.editing_id.as_deref(),
            label,
        )?,
        ParameterCatalog::ProcessingMethod => upsert_catalog_option(
            &mut state.coffee_parameters.processing_methods,
            "processing-method",
            input.editing_id.as_deref(),
            label,
        )?,
    };
    Ok(id)
}

pub fn upsert_roast_level_option(
    state: &mut AppState,
    input: &RoastLevelFormInput,
) -> Result<String, Vec<FormValidationError>> {
    let mut errors = Vec::new();
    let label = input.label.trim();
    if label.is_empty() {
        errors.push(form_error("label", "标签不能为空"));
    }
    if !label.is_empty()
        && has_duplicate_roast_level_label(
            &state.coffee_parameters.roast_levels,
            label,
            input.editing_id.as_deref(),
        )
    {
        errors.push(form_error("label", "未归档标签不能重复"));
    }

    let agtron_min = parse_agtron_bound_input(&input.agtron_min, "agtron_min", "下界", true, &mut errors);
    let agtron_max = parse_agtron_bound_input(&input.agtron_max, "agtron_max", "上界", false, &mut errors);

    if let (Some(min), Some(max)) = (agtron_min, agtron_max)
        && min > max
    {
        errors.push(form_error("agtron_max", "上界不能小于下界"));
    }

    if let Some(min) = agtron_min
        && has_duplicate_roast_level_range(
            &state.coffee_parameters.roast_levels,
            Some(min),
            agtron_max,
            input.editing_id.as_deref(),
        )
    {
        errors.push(form_error("agtron_max", "该 Agtron 范围已存在，请调整上下界"));
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let agtron_min = agtron_min.expect("agtron lower bound should exist after validation");
    let agtron_range = build_agtron_range(agtron_min, agtron_max);

    if let Some(editing_id) = input.editing_id.as_deref() {
        let Some(item) = state
            .coffee_parameters
            .roast_levels
            .iter_mut()
            .find(|item| item.id == editing_id)
        else {
            return Err(vec![form_error("editing_id", "未找到要更新的烘焙度")]);
        };
        item.label = label.to_string();
        item.agtron_range = agtron_range;
        item.agtron_min = Some(agtron_min);
        item.agtron_max = agtron_max;
        return Ok(item.id.clone());
    }

    let id = next_entity_id(
        "roast-level",
        state
            .coffee_parameters
            .roast_levels
            .iter()
            .map(|item| item.id.as_str()),
    );
    let next_sort_order = next_sort_order_roast_level(&state.coffee_parameters.roast_levels);
    state.coffee_parameters.roast_levels.push(RoastLevelOption {
        id: id.clone(),
        label: label.to_string(),
        agtron_range,
        agtron_min: Some(agtron_min),
        agtron_max,
        sort_order: next_sort_order,
        archived: false,
    });
    Ok(id)
}

pub fn upsert_coffee_bean(
    state: &mut AppState,
    input: &CoffeeBeanFormInput,
) -> Result<String, Vec<FormValidationError>> {
    let mut errors = Vec::new();
    let name = input.name.trim();
    if name.is_empty() {
        errors.push(form_error("name", "咖啡豆名称不能为空"));
    }

    let variety_id = normalize_optional_id(input.variety_id.as_deref());
    if let Some(variety_id) = variety_id.as_deref()
        && !contains_active_catalog_option(&state.coffee_parameters.bean_varieties, variety_id)
    {
        errors.push(form_error("variety_id", "豆种不存在或已归档"));
    }

    let processing_method_id = normalize_optional_id(input.processing_method_id.as_deref());
    if let Some(processing_method_id) = processing_method_id.as_deref()
        && !contains_active_catalog_option(
            &state.coffee_parameters.processing_methods,
            processing_method_id,
        )
    {
        errors.push(form_error("processing_method_id", "处理法不存在或已归档"));
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    if let Some(editing_id) = input.editing_id.as_deref() {
        let Some(bean) = state.beans.iter_mut().find(|item| item.id == editing_id) else {
            return Err(vec![form_error("editing_id", "未找到要更新的咖啡豆")]);
        };
        bean.name = name.to_string();
        bean.variety_id = variety_id;
        bean.processing_method_id = processing_method_id;
        bean.origin = blank_to_none(&input.origin);
        bean.notes = blank_to_none(&input.notes);
        return Ok(bean.id.clone());
    }

    let id = next_entity_id("bean", state.beans.iter().map(|item| item.id.as_str()));
    state.beans.push(CoffeeBean {
        id: id.clone(),
        name: name.to_string(),
        variety_id,
        processing_method_id,
        origin: blank_to_none(&input.origin),
        notes: blank_to_none(&input.notes),
        archived: false,
    });
    Ok(id)
}

pub fn upsert_roast_method(
    state: &mut AppState,
    input: &RoastMethodFormInput,
) -> Result<String, Vec<FormValidationError>> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(vec![form_error("name", "烘焙方法名称不能为空")]);
    }

    if let Some(editing_id) = input.editing_id.as_deref() {
        let Some(method) = state
            .roast_methods
            .iter_mut()
            .find(|item| item.id == editing_id)
        else {
            return Err(vec![form_error("editing_id", "未找到要更新的烘焙方法")]);
        };
        method.name = name.to_string();
        method.notes = blank_to_none(&input.notes);
        return Ok(method.id.clone());
    }

    let id = next_entity_id(
        "roast-method",
        state.roast_methods.iter().map(|item| item.id.as_str()),
    );
    state.roast_methods.push(RoastMethod {
        id: id.clone(),
        name: name.to_string(),
        notes: blank_to_none(&input.notes),
        archived: false,
    });
    Ok(id)
}

pub fn upsert_roast_profile(
    state: &mut AppState,
    input: &RoastProfileFormInput,
) -> Result<String, Vec<FormValidationError>> {
    let mut errors = Vec::new();
    if input.bean_id.trim().is_empty() {
        errors.push(form_error("bean_id", "请选择咖啡豆"));
    } else if !contains_active_bean(state, &input.bean_id) {
        errors.push(form_error("bean_id", "咖啡豆不存在或已归档"));
    }

    let method_id = normalize_optional_id(Some(input.method_id.as_str()));
    if let Some(method_id) = method_id.as_deref()
        && !contains_active_roast_method(state, method_id)
    {
        errors.push(form_error("method_id", "烘焙方法不存在或已归档"));
    }

    let roast_level_id = normalize_optional_id(input.roast_level_id.as_deref());
    if let Some(level_id) = roast_level_id.as_deref()
        && !contains_active_roast_level(state, level_id)
    {
        errors.push(form_error("roast_level_id", "烘焙度不存在或已归档"));
    }

    let batch_code = input.batch_code.trim();
    if batch_code.is_empty() {
        errors.push(form_error("batch_code", "batch_code 不能为空"));
    }

    let bean_name = state
        .beans
        .iter()
        .find(|bean| bean.id == input.bean_id)
        .map(|bean| bean.name.clone())
        .unwrap_or_default();
    if bean_name.is_empty() {
        errors.push(form_error("display_name", "无法生成烘焙品类名称"));
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let method_id = method_id.unwrap_or_else(|| ensure_default_roast_method(state));
    let method_name = resolve_roast_method_name(state, &method_id);
    let display_name = build_roast_profile_display_name(&bean_name, method_name, input.product_line);

    if let Some(editing_id) = input.editing_id.as_deref() {
        let Some(profile) = state
            .roast_profiles
            .iter_mut()
            .find(|item| item.id == editing_id)
        else {
            return Err(vec![form_error("editing_id", "未找到要更新的烘焙品类")]);
        };
        profile.bean_id = input.bean_id.clone();
        profile.method_id = method_id;
        profile.roast_level_id = roast_level_id;
        profile.product_line = input.product_line;
        profile.display_name = display_name;
        profile.batch_code = batch_code.to_string();
        return Ok(profile.id.clone());
    }

    let id = next_entity_id(
        "roast-profile",
        state.roast_profiles.iter().map(|item| item.id.as_str()),
    );
    state.roast_profiles.push(RoastProfile {
        id: id.clone(),
        bean_id: input.bean_id.clone(),
        method_id,
        roast_level_id,
        product_line: input.product_line,
        display_name,
        batch_code: batch_code.to_string(),
        recommended_rest_days: None,
        espresso_note: None,
        archived: false,
    });
    Ok(id)
}

pub fn upsert_brewing_plan_category(
    state: &mut AppState,
    input: &BrewingPlanCategoryFormInput,
) -> Result<String, Vec<FormValidationError>> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(vec![form_error("name", "分类名称不能为空")]);
    }

    if let Some(editing_id) = input.editing_id.as_deref() {
        let Some(category) = state
            .brewing_plan_categories
            .iter_mut()
            .find(|item| item.id == editing_id)
        else {
            return Err(vec![form_error("editing_id", "未找到要更新的分类")]);
        };
        category.name = name.to_string();
        return Ok(category.id.clone());
    }

    let id = next_entity_id(
        "plan-category",
        state
            .brewing_plan_categories
            .iter()
            .map(|item| item.id.as_str()),
    );
    let sort_order = state
        .brewing_plan_categories
        .iter()
        .map(|item| item.sort_order)
        .max()
        .unwrap_or(0)
        + 1;
    state.brewing_plan_categories.push(BrewingPlanCategory {
        id: id.clone(),
        name: name.to_string(),
        sort_order,
        plans: Vec::new(),
        archived: false,
    });
    Ok(id)
}

pub fn upsert_brewing_plan(
    state: &mut AppState,
    input: &BrewingPlanFormInput,
) -> Result<String, Vec<FormValidationError>> {
    let mut errors = Vec::new();
    if input.category_id.trim().is_empty() {
        errors.push(form_error("category_id", "请选择方案分类"));
    } else if !contains_active_category(state, &input.category_id) {
        errors.push(form_error("category_id", "分类不存在或已归档"));
    }
    if input.name.trim().is_empty() {
        errors.push(form_error("name", "方案名称不能为空"));
    }
    if input.matching_attributes.is_empty() {
        errors.push(form_error("matching_attributes", "至少添加一个匹配属性"));
    } else {
        for (index, attr) in input.matching_attributes.iter().enumerate() {
            if !match_attribute_exists(state, attr) {
                errors.push(form_error(
                    &format!("matching_attributes[{index}]"),
                    "匹配属性引用不存在或已归档",
                ));
            }
        }
    }
    if input.pour_stages == 0 {
        errors.push(form_error("pour_stages", "注水段数必须大于 0"));
    }
    if input.dripper.trim().is_empty() {
        errors.push(form_error("dripper", "滤杯不能为空"));
    }
    if input.ratio_coffee <= 0.0 || input.ratio_water <= 0.0 {
        errors.push(form_error("ratio", "比例必须大于 0"));
    }
    if input.default_dose_g <= 0.0 {
        errors.push(form_error("default_dose_g", "默认粉量必须大于 0"));
    }
    if input.day0_grind_size <= 0.0
        || input.day0_water_temp_c <= 0.0
        || input.day14_grind_size <= 0.0
        || input.day14_water_temp_c <= 0.0
    {
        errors.push(form_error(
            "age_fitting",
            "day0/day14 研磨度和水温必须大于 0",
        ));
    }
    let grinder_profile_id = normalize_optional_id(input.grinder_profile_id.as_deref());
    if let Some(grinder_profile_id) = grinder_profile_id.as_deref()
        && !contains_active_grinder(state, grinder_profile_id)
    {
        errors.push(form_error("grinder_profile_id", "磨豆机不存在或已归档"));
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let target_category_index = state
        .brewing_plan_categories
        .iter()
        .position(|category| category.id == input.category_id)
        .expect("category already validated");

    let plan_id = if let Some(editing_id) = input.editing_id.as_deref() {
        editing_id.to_string()
    } else {
        next_entity_id(
            "brewing-plan",
            state
                .brewing_plan_categories
                .iter()
                .flat_map(|category| category.plans.iter().map(|plan| plan.id.as_str())),
        )
    };
    let archived = input
        .editing_id
        .as_deref()
        .and_then(|editing_id| {
            state
                .brewing_plan_categories
                .iter()
                .flat_map(|category| category.plans.iter())
                .find(|plan| plan.id == editing_id)
                .map(|plan| plan.archived)
        })
        .unwrap_or(false);

    let updated_plan = BrewingPlan {
        id: plan_id.clone(),
        name: input.name.trim().to_string(),
        matching_attributes: input.matching_attributes.clone(),
        parameters: BrewingPlanParameters {
            pour_stages: input.pour_stages,
            dripper: input.dripper.trim().to_string(),
            grinder_profile_id,
            ratio: BrewRatio {
                coffee: input.ratio_coffee,
                water: input.ratio_water,
            },
            default_dose_g: input.default_dose_g,
        },
        age_fitting: BrewingAgeFitting {
            day0: BrewingAgeEndpoint {
                grind_size: input.day0_grind_size,
                water_temp_c: input.day0_water_temp_c,
            },
            day14: BrewingAgeEndpoint {
                grind_size: input.day14_grind_size,
                water_temp_c: input.day14_water_temp_c,
            },
        },
        instructions: blank_to_none(&input.instructions),
        priority: input.priority,
        archived,
    };

    if let Some(editing_id) = input.editing_id.as_deref() {
        let Some((current_category_index, current_plan_index)) = find_plan_index(state, editing_id)
        else {
            return Err(vec![form_error("editing_id", "未找到要更新的冲煮方案")]);
        };
        if current_category_index == target_category_index {
            state.brewing_plan_categories[current_category_index].plans[current_plan_index] =
                updated_plan;
            return Ok(plan_id);
        }

        state.brewing_plan_categories[current_category_index]
            .plans
            .remove(current_plan_index);
        state.brewing_plan_categories[target_category_index]
            .plans
            .push(updated_plan);
        return Ok(plan_id);
    }

    state.brewing_plan_categories[target_category_index]
        .plans
        .push(updated_plan);
    Ok(plan_id)
}

pub fn add_matching_attribute(
    attributes: &mut Vec<BrewingMatchAttribute>,
    kind: BrewingMatchKind,
    option_id: &str,
) -> bool {
    let normalized_option_id = option_id.trim();
    if normalized_option_id.is_empty() {
        return false;
    }
    if attributes
        .iter()
        .any(|attr| attr.kind == kind && attr.option_id == normalized_option_id)
    {
        return false;
    }
    attributes.push(BrewingMatchAttribute {
        kind,
        option_id: normalized_option_id.to_string(),
    });
    true
}

pub fn remove_matching_attribute(
    attributes: &mut Vec<BrewingMatchAttribute>,
    index: usize,
) -> bool {
    if index >= attributes.len() {
        return false;
    }
    attributes.remove(index);
    true
}

pub fn can_write_catalog(pending_archive: Option<&PendingArchive>) -> bool {
    pending_archive.is_none()
}

pub fn begin_pending_archive(
    pending: &Option<PendingArchive>,
    target: ArchiveTarget,
) -> Result<PendingArchive, PendingArchiveError> {
    if pending.is_some() {
        return Err(PendingArchiveError::AlreadyPending);
    }
    Ok(PendingArchive {
        target,
        remaining_seconds: 5,
    })
}

pub fn cancel_pending_archive(pending: &mut Option<PendingArchive>) -> bool {
    if pending.is_none() {
        return false;
    }
    *pending = None;
    true
}

pub fn commit_pending_archive(
    state: &mut AppState,
    pending: PendingArchive,
) -> Result<(), ArchiveCommitError> {
    match pending.target {
        ArchiveTarget::BeanVariety { id } => {
            archive_catalog_option(&mut state.coffee_parameters.bean_varieties, &id)
        }
        ArchiveTarget::RoastLevel { id } => {
            archive_roast_level_option(&mut state.coffee_parameters.roast_levels, &id)
        }
        ArchiveTarget::ProcessingMethod { id } => {
            archive_catalog_option(&mut state.coffee_parameters.processing_methods, &id)
        }
        ArchiveTarget::CoffeeBean { id } => archive_coffee_bean(state, &id),
        ArchiveTarget::RoastMethod { id } => archive_roast_method(state, &id),
        ArchiveTarget::RoastProfile { id } => archive_roast_profile(state, &id),
        ArchiveTarget::BrewingPlanCategory { id } => archive_brewing_plan_category(state, &id),
        ArchiveTarget::BrewingPlan { category_id, id } => {
            archive_brewing_plan(state, &category_id, &id)
        }
        ArchiveTarget::BatchUsedUp { id } => mark_batch_used_up(state, &id),
        ArchiveTarget::BatchArchived { id } => archive_batch(state, &id),
    }
}

pub fn pending_archive_label(state: &AppState, pending: &PendingArchive) -> String {
    match &pending.target {
        ArchiveTarget::BeanVariety { id } => {
            lookup_catalog_label(&state.coffee_parameters.bean_varieties, id, "豆种")
        }
        ArchiveTarget::RoastLevel { id } => {
            lookup_roast_level_label(&state.coffee_parameters.roast_levels, id)
        }
        ArchiveTarget::ProcessingMethod { id } => {
            lookup_catalog_label(&state.coffee_parameters.processing_methods, id, "处理法")
        }
        ArchiveTarget::CoffeeBean { id } => lookup_name(
            state.beans.iter().map(|item| (&item.id, &item.name)),
            id,
            "咖啡豆",
        ),
        ArchiveTarget::RoastMethod { id } => lookup_name(
            state
                .roast_methods
                .iter()
                .map(|item| (&item.id, &item.name)),
            id,
            "烘焙方法",
        ),
        ArchiveTarget::RoastProfile { id } => lookup_name(
            state
                .roast_profiles
                .iter()
                .map(|item| (&item.id, &item.display_name)),
            id,
            "烘焙品类",
        ),
        ArchiveTarget::BrewingPlanCategory { id } => lookup_name(
            state
                .brewing_plan_categories
                .iter()
                .map(|item| (&item.id, &item.name)),
            id,
            "方案分类",
        ),
        ArchiveTarget::BrewingPlan { category_id, id } => {
            let label = state
                .brewing_plan_categories
                .iter()
                .find(|category| category.id == *category_id)
                .and_then(|category| {
                    category
                        .plans
                        .iter()
                        .find(|plan| plan.id == *id)
                        .map(|plan| plan.name.as_str())
                })
                .unwrap_or("冲煮方案");
            format!("冲煮方案：{label}")
        }
        ArchiveTarget::BatchUsedUp { id } => lookup_batch_label(state, id, "标记用完"),
        ArchiveTarget::BatchArchived { id } => lookup_batch_label(state, id, "归档"),
    }
}

fn upsert_catalog_option(
    items: &mut Vec<CatalogOption>,
    id_prefix: &str,
    editing_id: Option<&str>,
    label: &str,
) -> Result<String, Vec<FormValidationError>> {
    if let Some(editing_id) = editing_id {
        let Some(item) = items.iter_mut().find(|item| item.id == editing_id) else {
            return Err(vec![form_error("editing_id", "未找到要更新的目录项")]);
        };
        item.label = label.to_string();
        return Ok(item.id.clone());
    }
    let id = next_entity_id(id_prefix, items.iter().map(|item| item.id.as_str()));
    let sort_order = items.iter().map(|item| item.sort_order).max().unwrap_or(0) + 1;
    items.push(CatalogOption {
        id: id.clone(),
        label: label.to_string(),
        sort_order,
        archived: false,
    });
    Ok(id)
}

fn has_duplicate_catalog_label(
    items: &[CatalogOption],
    label: &str,
    ignore_id: Option<&str>,
) -> bool {
    items
        .iter()
        .filter(|item| !item.archived)
        .any(|item| item.label == label && Some(item.id.as_str()) != ignore_id)
}

fn has_duplicate_roast_level_label(
    items: &[RoastLevelOption],
    label: &str,
    ignore_id: Option<&str>,
) -> bool {
    items
        .iter()
        .filter(|item| !item.archived)
        .any(|item| item.label == label && Some(item.id.as_str()) != ignore_id)
}

fn has_duplicate_roast_level_range(
    items: &[RoastLevelOption],
    min: Option<f32>,
    max: Option<f32>,
    ignore_id: Option<&str>,
) -> bool {
    items
        .iter()
        .filter(|item| !item.archived)
        .filter(|item| Some(item.id.as_str()) != ignore_id)
        .filter_map(|item| roast_level_bounds(item).map(|bounds| (item.id.as_str(), bounds)))
        .any(|(_, bounds)| bounds == (min, max))
}

fn parse_agtron_bound_input(
    raw: &str,
    field: &str,
    label: &str,
    required: bool,
    errors: &mut Vec<FormValidationError>,
) -> Option<f32> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        if required {
            errors.push(form_error(field, &format!("Agtron {label}不能为空")));
        }
        return None;
    }

    let Ok(value) = trimmed.parse::<f32>() else {
        errors.push(form_error(field, &format!("Agtron {label}请输入数字")));
        return None;
    };

    if !value.is_finite() || !(1.0..=150.0).contains(&value) {
        errors.push(form_error(field, &format!("Agtron {label}需在 1-150 之间")));
        return None;
    }

    Some(value)
}

fn build_agtron_range(min: f32, max: Option<f32>) -> String {
    match max {
        Some(max) if (min - max).abs() < f32::EPSILON => format_agtron_value(min),
        Some(max) => format!("{}-{}", format_agtron_value(min), format_agtron_value(max)),
        None => format!("{}+", format_agtron_value(min)),
    }
}

fn format_agtron_value(value: f32) -> String {
    let rounded = (value * 10.0).round() / 10.0;
    if (rounded.fract()).abs() < f32::EPSILON {
        format!("{rounded:.0}")
    } else {
        format!("{rounded:.1}")
    }
}

fn next_sort_order_roast_level(items: &[RoastLevelOption]) -> u32 {
    items.iter().map(|item| item.sort_order).max().unwrap_or(0) + 1
}

fn normalize_optional_id(id: Option<&str>) -> Option<String> {
    id.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn blank_to_none(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn form_error(field: &str, message: &str) -> FormValidationError {
    FormValidationError {
        field: field.to_string(),
        message: message.to_string(),
    }
}

fn contains_active_catalog_option(items: &[CatalogOption], id: &str) -> bool {
    items.iter().any(|item| item.id == id && !item.archived)
}

fn contains_active_roast_level(state: &AppState, id: &str) -> bool {
    state
        .coffee_parameters
        .roast_levels
        .iter()
        .any(|item| item.id == id && !item.archived)
}

fn contains_active_bean(state: &AppState, id: &str) -> bool {
    state
        .beans
        .iter()
        .any(|item| item.id == id && !item.archived)
}

fn contains_active_roast_method(state: &AppState, id: &str) -> bool {
    state
        .roast_methods
        .iter()
        .any(|item| item.id == id && !item.archived)
}

fn contains_active_grinder(state: &AppState, id: &str) -> bool {
    state
        .grinder_profiles
        .iter()
        .any(|item| item.id == id && !item.archived)
}

fn contains_active_category(state: &AppState, id: &str) -> bool {
    state
        .brewing_plan_categories
        .iter()
        .any(|item| item.id == id && !item.archived)
}

fn match_attribute_exists(state: &AppState, attr: &BrewingMatchAttribute) -> bool {
    match attr.kind {
        BrewingMatchKind::BeanVariety => {
            contains_active_catalog_option(&state.coffee_parameters.bean_varieties, &attr.option_id)
        }
        BrewingMatchKind::ProcessingMethod => contains_active_catalog_option(
            &state.coffee_parameters.processing_methods,
            &attr.option_id,
        ),
        BrewingMatchKind::RoastLevel => contains_active_roast_level(state, &attr.option_id),
    }
}

fn find_plan_index(state: &AppState, plan_id: &str) -> Option<(usize, usize)> {
    state
        .brewing_plan_categories
        .iter()
        .enumerate()
        .find_map(|(category_index, category)| {
            category
                .plans
                .iter()
                .position(|plan| plan.id == plan_id)
                .map(|plan_index| (category_index, plan_index))
        })
}

fn archive_catalog_option(items: &mut [CatalogOption], id: &str) -> Result<(), ArchiveCommitError> {
    let Some(item) = items.iter_mut().find(|item| item.id == id) else {
        return Err(ArchiveCommitError::TargetNotFound);
    };
    item.archived = true;
    Ok(())
}

fn archive_roast_level_option(
    items: &mut [RoastLevelOption],
    id: &str,
) -> Result<(), ArchiveCommitError> {
    let Some(item) = items.iter_mut().find(|item| item.id == id) else {
        return Err(ArchiveCommitError::TargetNotFound);
    };
    item.archived = true;
    Ok(())
}

fn archive_coffee_bean(state: &mut AppState, id: &str) -> Result<(), ArchiveCommitError> {
    let Some(item) = state.beans.iter_mut().find(|item| item.id == id) else {
        return Err(ArchiveCommitError::TargetNotFound);
    };
    item.archived = true;
    Ok(())
}

fn archive_roast_method(state: &mut AppState, id: &str) -> Result<(), ArchiveCommitError> {
    let Some(item) = state.roast_methods.iter_mut().find(|item| item.id == id) else {
        return Err(ArchiveCommitError::TargetNotFound);
    };
    item.archived = true;
    Ok(())
}

fn archive_roast_profile(state: &mut AppState, id: &str) -> Result<(), ArchiveCommitError> {
    let Some(item) = state.roast_profiles.iter_mut().find(|item| item.id == id) else {
        return Err(ArchiveCommitError::TargetNotFound);
    };
    item.archived = true;
    Ok(())
}

fn archive_brewing_plan_category(state: &mut AppState, id: &str) -> Result<(), ArchiveCommitError> {
    let Some(item) = state
        .brewing_plan_categories
        .iter_mut()
        .find(|item| item.id == id)
    else {
        return Err(ArchiveCommitError::TargetNotFound);
    };
    item.archived = true;
    Ok(())
}

fn archive_brewing_plan(
    state: &mut AppState,
    category_id: &str,
    id: &str,
) -> Result<(), ArchiveCommitError> {
    let Some(category) = state
        .brewing_plan_categories
        .iter_mut()
        .find(|item| item.id == category_id)
    else {
        return Err(ArchiveCommitError::TargetNotFound);
    };
    let Some(plan) = category.plans.iter_mut().find(|item| item.id == id) else {
        return Err(ArchiveCommitError::TargetNotFound);
    };
    plan.archived = true;
    Ok(())
}

fn mark_batch_used_up(state: &mut AppState, id: &str) -> Result<(), ArchiveCommitError> {
    let Some(batch) = state.batches.iter_mut().find(|item| item.id == id) else {
        return Err(ArchiveCommitError::TargetNotFound);
    };
    batch.status = crate::domain::models::BatchStatus::UsedUp;
    Ok(())
}

fn archive_batch(state: &mut AppState, id: &str) -> Result<(), ArchiveCommitError> {
    let Some(batch) = state.batches.iter_mut().find(|item| item.id == id) else {
        return Err(ArchiveCommitError::TargetNotFound);
    };
    batch.status = crate::domain::models::BatchStatus::Archived;
    Ok(())
}

fn lookup_catalog_label(items: &[CatalogOption], id: &str, prefix: &str) -> String {
    let label = items
        .iter()
        .find(|item| item.id == id)
        .map(|item| item.label.as_str())
        .unwrap_or(prefix);
    format!("{prefix}：{label}")
}

fn lookup_roast_level_label(items: &[RoastLevelOption], id: &str) -> String {
    let label = items
        .iter()
        .find(|item| item.id == id)
        .map(|item| item.label.as_str())
        .unwrap_or("烘焙度");
    format!("烘焙度：{label}")
}

fn lookup_name<'a>(
    mut items: impl Iterator<Item = (&'a String, &'a String)>,
    id: &str,
    prefix: &str,
) -> String {
    let label = items
        .find(|(item_id, _)| item_id.as_str() == id)
        .map(|(_, name)| name.as_str())
        .unwrap_or(prefix);
    format!("{prefix}：{label}")
}

fn lookup_batch_label(state: &AppState, id: &str, prefix: &str) -> String {
    let label = state
        .batches
        .iter()
        .find(|item| item.id == id)
        .map(|item| item.batch_no.as_str())
        .unwrap_or("批次");
    format!("{prefix}：{label}")
}

fn ensure_default_roast_method(state: &mut AppState) -> String {
    if let Some(method) = state
        .roast_methods
        .iter()
        .find(|item| item.id == DEFAULT_ROAST_METHOD_ID && !item.archived)
    {
        return method.id.clone();
    }

    state.roast_methods.push(RoastMethod {
        id: DEFAULT_ROAST_METHOD_ID.to_string(),
        name: DEFAULT_ROAST_METHOD_NAME.to_string(),
        notes: Some("系统默认烘焙流程".to_string()),
        archived: false,
    });
    DEFAULT_ROAST_METHOD_ID.to_string()
}

fn resolve_roast_method_name<'a>(state: &'a AppState, method_id: &str) -> &'a str {
    state
        .roast_methods
        .iter()
        .find(|item| item.id == method_id)
        .map(|item| item.name.as_str())
        .unwrap_or(DEFAULT_ROAST_METHOD_NAME)
}

fn build_roast_profile_display_name(
    bean_name: &str,
    method_name: &str,
    product_line: ProductLine,
) -> String {
    if method_name == DEFAULT_ROAST_METHOD_NAME {
        format!("{} {}", bean_name, product_line_label(product_line))
    } else {
        format!("{} {} {}", bean_name, method_name, product_line_label(product_line))
    }
}

fn product_line_label(product_line: ProductLine) -> &'static str {
    match product_line {
        ProductLine::PourOver => "手冲",
        ProductLine::Espresso => "意式",
    }
}

fn alnum_prefix(value: &str, length: usize) -> String {
    let mut chars = value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_uppercase())
        .take(length)
        .collect::<String>();
    if chars.len() < length {
        chars.extend(std::iter::repeat_n('X', length - chars.len()));
    }
    chars
}

fn next_entity_id<'a>(prefix: &str, ids: impl Iterator<Item = &'a str>) -> String {
    let mut max_seq = 0_u32;
    let needle = format!("{prefix}-");
    for id in ids {
        if let Some(suffix) = id.strip_prefix(&needle)
            && let Ok(seq) = suffix.parse::<u32>()
        {
            max_seq = max_seq.max(seq);
        }
    }
    format!("{prefix}-{:04}", max_seq + 1)
}

#[cfg(test)]
mod tests {
    use crate::domain::models::{BrewingMatchAttribute, BrewingMatchKind, ProductLine};
    use crate::domain::seed::seed_app_state;

    use super::{
        ArchiveTarget, BrewingPlanCategoryFormInput, BrewingPlanFormInput, CatalogOptionFormInput,
        CoffeeBeanFormInput, ParameterCatalog, RoastLevelFormInput, RoastMethodFormInput,
        RoastProfileFormInput, add_matching_attribute, begin_pending_archive,
        cancel_pending_archive, commit_pending_archive, pending_archive_label,
        remove_matching_attribute, suggest_batch_code, upsert_brewing_plan,
        upsert_brewing_plan_category, upsert_coffee_bean, upsert_parameter_option,
        upsert_roast_level_option, upsert_roast_method, upsert_roast_profile,
    };

    #[test]
    fn suggest_batch_code_builds_code_from_bean_and_method_names() {
        let code = suggest_batch_code("Yirgacheffe", "Fast Curve", ProductLine::PourOver);
        assert_eq!(code, "YIFAPO");
    }

    #[test]
    fn suggest_batch_code_uses_fallback_prefix_when_name_has_no_ascii() {
        let code = suggest_batch_code("耶加雪菲", "标准曲线", ProductLine::Espresso);
        assert_eq!(code, "XXXXES");
    }

    #[test]
    fn upsert_coffee_bean_returns_validation_errors_for_missing_name_and_invalid_refs() {
        let mut state = seed_app_state();
        let input = CoffeeBeanFormInput {
            editing_id: None,
            name: String::new(),
            variety_id: Some("missing-variety".to_string()),
            processing_method_id: Some("missing-processing".to_string()),
            origin: String::new(),
            notes: String::new(),
        };

        let errors = upsert_coffee_bean(&mut state, &input).expect_err("bean save should fail");
        let fields = errors
            .iter()
            .map(|error| error.field.as_str())
            .collect::<Vec<_>>();
        assert_eq!(fields, vec!["name", "variety_id", "processing_method_id"]);
    }

    #[test]
    fn upsert_coffee_bean_add_and_update_changes_state() {
        let mut state = seed_app_state();
        let created_id = upsert_coffee_bean(
            &mut state,
            &CoffeeBeanFormInput {
                editing_id: None,
                name: "Yirgacheffe G1".to_string(),
                variety_id: Some("bean-var-ethiopian-heirloom".to_string()),
                processing_method_id: Some("process-washed".to_string()),
                origin: "Ethiopia".to_string(),
                notes: "floral".to_string(),
            },
        )
        .expect("bean create should succeed");

        let updated_id = upsert_coffee_bean(
            &mut state,
            &CoffeeBeanFormInput {
                editing_id: Some(created_id.clone()),
                name: "Yirgacheffe G2".to_string(),
                variety_id: Some("bean-var-bourbon".to_string()),
                processing_method_id: Some("process-honey".to_string()),
                origin: "Sidama".to_string(),
                notes: "citrus".to_string(),
            },
        )
        .expect("bean update should succeed");

        assert_eq!(updated_id, created_id);
        assert_eq!(state.beans.len(), 1);
        assert_eq!(state.beans[0].name, "Yirgacheffe G2");
        assert_eq!(
            state.beans[0].variety_id.as_deref(),
            Some("bean-var-bourbon")
        );
        assert_eq!(
            state.beans[0].processing_method_id.as_deref(),
            Some("process-honey")
        );
        assert_eq!(state.beans[0].origin.as_deref(), Some("Sidama"));
        assert_eq!(state.beans[0].notes.as_deref(), Some("citrus"));
    }

    #[test]
    fn upsert_parameter_option_and_roast_level_add_items() {
        let mut state = seed_app_state();

        let variety_id = upsert_parameter_option(
            &mut state,
            ParameterCatalog::BeanVariety,
            &CatalogOptionFormInput {
                editing_id: None,
                label: "SL28".to_string(),
            },
        )
        .expect("variety create should succeed");

        let roast_level_id = upsert_roast_level_option(
            &mut state,
            &RoastLevelFormInput {
                editing_id: None,
                label: "超浅".to_string(),
                agtron_min: "96".to_string(),
                agtron_max: String::new(),
            },
        )
        .expect("roast level create should succeed");

        assert_eq!(variety_id, "bean-variety-0001");
        assert_eq!(roast_level_id, "roast-level-0001");
        assert_eq!(
            state
                .coffee_parameters
                .bean_varieties
                .iter()
                .map(|item| item.label.as_str())
                .collect::<Vec<_>>()
                .last()
                .copied(),
            Some("SL28")
        );
        assert_eq!(
            state
                .coffee_parameters
                .roast_levels
                .iter()
                .map(|item| item.label.as_str())
                .collect::<Vec<_>>()
                .last()
                .copied(),
            Some("超浅")
        );
        let created_level = state
            .coffee_parameters
            .roast_levels
            .iter()
            .find(|item| item.id == roast_level_id)
            .expect("created roast level should exist");
        assert_eq!(created_level.agtron_min, Some(96.0));
        assert_eq!(created_level.agtron_max, None);
    }

    #[test]
    fn upsert_roast_level_rejects_duplicate_range() {
        let mut state = seed_app_state();
        let errors = upsert_roast_level_option(
            &mut state,
            &RoastLevelFormInput {
                editing_id: None,
                label: "重复浅烘".to_string(),
                agtron_min: "90".to_string(),
                agtron_max: "95".to_string(),
            },
        )
        .expect_err("duplicate roast level range should fail");

        assert!(errors
            .iter()
            .any(|error| error.field == "agtron_max" && error.message.contains("已存在")));
    }

    #[test]
    fn upsert_roast_method_and_roast_profile_create_records() {
        let mut state = seed_app_state();
        let bean_id = upsert_coffee_bean(
            &mut state,
            &CoffeeBeanFormInput {
                editing_id: None,
                name: "Guji".to_string(),
                variety_id: Some("bean-var-ethiopian-heirloom".to_string()),
                processing_method_id: Some("process-washed".to_string()),
                origin: String::new(),
                notes: String::new(),
            },
        )
        .expect("bean create should succeed");
        let method_id = upsert_roast_method(
            &mut state,
            &RoastMethodFormInput {
                editing_id: None,
                name: "Fast Curve".to_string(),
                notes: String::new(),
            },
        )
        .expect("method create should succeed");

        let profile_id = upsert_roast_profile(
            &mut state,
            &RoastProfileFormInput {
                editing_id: None,
                bean_id,
                method_id,
                roast_level_id: Some("roast-level-light".to_string()),
                product_line: ProductLine::PourOver,
                batch_code: "GUFCPO".to_string(),
            },
        )
        .expect("profile create should succeed");

        assert_eq!(profile_id, "roast-profile-0001");
        assert_eq!(state.roast_profiles.len(), 1);
        assert_eq!(state.roast_profiles[0].display_name, "Guji Fast Curve 手冲");
        assert_eq!(state.roast_profiles[0].batch_code, "GUFCPO");
    }

    #[test]
    fn upsert_roast_profile_rejects_empty_batch_code() {
        let mut state = seed_app_state();
        let bean_id = upsert_coffee_bean(
            &mut state,
            &CoffeeBeanFormInput {
                editing_id: None,
                name: "Guji".to_string(),
                variety_id: Some("bean-var-ethiopian-heirloom".to_string()),
                processing_method_id: Some("process-washed".to_string()),
                origin: String::new(),
                notes: String::new(),
            },
        )
        .expect("bean create should succeed");
        let method_id = upsert_roast_method(
            &mut state,
            &RoastMethodFormInput {
                editing_id: None,
                name: "Fast Curve".to_string(),
                notes: String::new(),
            },
        )
        .expect("method create should succeed");

        let errors = upsert_roast_profile(
            &mut state,
            &RoastProfileFormInput {
                editing_id: None,
                bean_id,
                method_id,
                roast_level_id: Some("roast-level-light".to_string()),
                product_line: ProductLine::PourOver,
                batch_code: String::new(),
            },
        )
        .expect_err("profile save should fail");
        let fields = errors
            .iter()
            .map(|error| error.field.as_str())
            .collect::<Vec<_>>();
        assert_eq!(fields, vec!["batch_code"]);
    }

    #[test]
    fn add_and_remove_matching_attributes_edits_attribute_list() {
        let mut attributes = vec![BrewingMatchAttribute {
            kind: BrewingMatchKind::ProcessingMethod,
            option_id: "process-washed".to_string(),
        }];

        let first_added = add_matching_attribute(
            &mut attributes,
            BrewingMatchKind::RoastLevel,
            "roast-level-dark",
        );
        let second_added = add_matching_attribute(
            &mut attributes,
            BrewingMatchKind::RoastLevel,
            "roast-level-dark",
        );
        let removed = remove_matching_attribute(&mut attributes, 1);

        assert_eq!(first_added, true);
        assert_eq!(second_added, false);
        assert_eq!(removed, true);
        assert_eq!(
            attributes,
            vec![BrewingMatchAttribute {
                kind: BrewingMatchKind::ProcessingMethod,
                option_id: "process-washed".to_string(),
            }]
        );
    }

    #[test]
    fn upsert_brewing_plan_can_move_plan_between_categories() {
        let mut state = seed_app_state();
        let added_category_id = upsert_brewing_plan_category(
            &mut state,
            &BrewingPlanCategoryFormInput {
                editing_id: None,
                name: "测试分类".to_string(),
            },
        )
        .expect("category create should succeed");
        let original_plan = state.brewing_plan_categories[0].plans[0].clone();

        let updated_id = upsert_brewing_plan(
            &mut state,
            &BrewingPlanFormInput {
                editing_id: Some(original_plan.id.clone()),
                category_id: added_category_id.clone(),
                name: "迁移方案".to_string(),
                matching_attributes: vec![BrewingMatchAttribute {
                    kind: BrewingMatchKind::ProcessingMethod,
                    option_id: "process-washed".to_string(),
                }],
                pour_stages: 2,
                dripper: "V60".to_string(),
                grinder_profile_id: Some("grinder-ditting".to_string()),
                ratio_coffee: 1.0,
                ratio_water: 16.0,
                default_dose_g: 16.0,
                day0_grind_size: 6.0,
                day0_water_temp_c: 93.0,
                day14_grind_size: 6.4,
                day14_water_temp_c: 91.0,
                instructions: "test".to_string(),
                priority: 1,
            },
        )
        .expect("plan update should succeed");

        let source_plan_ids = state.brewing_plan_categories[0]
            .plans
            .iter()
            .map(|plan| plan.id.as_str())
            .collect::<Vec<_>>();
        let target_plans = state
            .brewing_plan_categories
            .iter()
            .find(|category| category.id == added_category_id)
            .map(|category| {
                category
                    .plans
                    .iter()
                    .map(|plan| plan.name.as_str())
                    .collect::<Vec<_>>()
            })
            .expect("category should exist");

        assert_eq!(updated_id, original_plan.id);
        assert_eq!(source_plan_ids.contains(&original_plan.id.as_str()), false);
        assert_eq!(target_plans, vec!["迁移方案"]);
    }

    #[test]
    fn pending_archive_can_be_started_and_canceled_without_mutating_state() {
        let state = seed_app_state();
        let pending = begin_pending_archive(
            &None,
            ArchiveTarget::ProcessingMethod {
                id: "process-washed".to_string(),
            },
        )
        .expect("pending should start");

        let label = pending_archive_label(&state, &pending);
        let mut pending_slot = Some(pending);
        let canceled = cancel_pending_archive(&mut pending_slot);

        assert_eq!(label, "处理法：水洗");
        assert_eq!(canceled, true);
        assert_eq!(pending_slot, None);
    }

    #[test]
    fn commit_pending_archive_marks_target_archived() {
        let mut state = seed_app_state();
        let pending = begin_pending_archive(
            &None,
            ArchiveTarget::ProcessingMethod {
                id: "process-washed".to_string(),
            },
        )
        .expect("pending should start");

        commit_pending_archive(&mut state, pending).expect("archive commit should succeed");

        let archived_values = state
            .coffee_parameters
            .processing_methods
            .iter()
            .filter(|item| item.id == "process-washed")
            .map(|item| item.archived)
            .collect::<Vec<_>>();
        assert_eq!(archived_values, vec![true]);
    }

    #[test]
    fn begin_pending_archive_rejects_when_another_archive_exists() {
        let existing = Some(
            begin_pending_archive(
                &None,
                ArchiveTarget::CoffeeBean {
                    id: "bean-1".to_string(),
                },
            )
            .expect("existing pending should start"),
        );

        let result = begin_pending_archive(
            &existing,
            ArchiveTarget::RoastMethod {
                id: "method-1".to_string(),
            },
        );

        assert_eq!(
            result.expect_err("should reject second pending archive"),
            super::PendingArchiveError::AlreadyPending
        );
    }

    #[test]
    fn can_write_catalog_returns_true_when_no_pending_archive() {
        assert_eq!(super::can_write_catalog(None), true);
    }

    #[test]
    fn can_write_catalog_returns_false_when_pending_archive_exists() {
        let pending = super::PendingArchive {
            target: super::ArchiveTarget::CoffeeBean {
                id: "bean-1".to_string(),
            },
            remaining_seconds: 5,
        };
        assert_eq!(super::can_write_catalog(Some(&pending)), false);
    }

    #[test]
    fn commit_pending_archive_marks_batch_used_up() {
        let mut state = seed_app_state();
        state.batches.push(crate::domain::models::RoastBatch {
            id: "batch-usedup-test".to_string(),
            profile_id: "profile-1".to_string(),
            bean_id: "bean-1".to_string(),
            product_line: Some(ProductLine::PourOver),
            roast_level_id: Some("roast-level-light".to_string()),
            batch_code: "TEST".to_string(),
            roasted_at: "2026-05-02T08:00:00Z".to_string(),
            batch_no: "20260502-TEST-001".to_string(),
            status: crate::domain::models::BatchStatus::Active,
            agtron_score: None,
            matched_roast_level_id: None,
            notes: None,
            capacity_g: 100.0,
        });
        let pending = begin_pending_archive(
            &None,
            super::ArchiveTarget::BatchUsedUp {
                id: "batch-usedup-test".to_string(),
            },
        )
        .expect("pending should start");

        commit_pending_archive(&mut state, pending).expect("commit should succeed");

        assert_eq!(
            state.batches[0].status,
            crate::domain::models::BatchStatus::UsedUp
        );
    }

    #[test]
    fn commit_pending_archive_marks_batch_archived() {
        let mut state = seed_app_state();
        state.batches.push(crate::domain::models::RoastBatch {
            id: "batch-archive-test".to_string(),
            profile_id: "profile-1".to_string(),
            bean_id: "bean-1".to_string(),
            product_line: Some(ProductLine::PourOver),
            roast_level_id: Some("roast-level-light".to_string()),
            batch_code: "TEST".to_string(),
            roasted_at: "2026-05-02T08:00:00Z".to_string(),
            batch_no: "20260502-TEST-002".to_string(),
            status: crate::domain::models::BatchStatus::Active,
            agtron_score: None,
            matched_roast_level_id: None,
            notes: None,
            capacity_g: 100.0,
        });
        let pending = begin_pending_archive(
            &None,
            super::ArchiveTarget::BatchArchived {
                id: "batch-archive-test".to_string(),
            },
        )
        .expect("pending should start");

        commit_pending_archive(&mut state, pending).expect("commit should succeed");

        assert_eq!(
            state.batches[0].status,
            crate::domain::models::BatchStatus::Archived
        );
    }

    #[test]
    fn can_write_catalog_returns_false_for_batch_used_up_pending() {
        let pending = super::PendingArchive {
            target: super::ArchiveTarget::BatchUsedUp {
                id: "batch-1".to_string(),
            },
            remaining_seconds: 5,
        };
        assert_eq!(super::can_write_catalog(Some(&pending)), false);
    }

    #[test]
    fn pending_archive_label_shows_batch_used_up() {
        let mut state = seed_app_state();
        state.batches.push(crate::domain::models::RoastBatch {
            id: "batch-001".to_string(),
            profile_id: "profile-1".to_string(),
            bean_id: "bean-1".to_string(),
            product_line: Some(ProductLine::PourOver),
            roast_level_id: Some("roast-level-light".to_string()),
            batch_code: "TEST".to_string(),
            roasted_at: "2026-05-02T08:00:00Z".to_string(),
            batch_no: "20260502-TEST-001".to_string(),
            status: crate::domain::models::BatchStatus::Active,
            agtron_score: None,
            matched_roast_level_id: None,
            notes: None,
            capacity_g: 100.0,
        });
        let pending = super::PendingArchive {
            target: super::ArchiveTarget::BatchUsedUp {
                id: "batch-001".to_string(),
            },
            remaining_seconds: 5,
        };
        let label = super::pending_archive_label(&state, &pending);
        assert_eq!(label, "标记用完：20260502-TEST-001");
    }

    #[test]
    fn pending_archive_initial_remaining_seconds_is_five() {
        let pending = begin_pending_archive(
            &None,
            super::ArchiveTarget::CoffeeBean {
                id: "bean-1".to_string(),
            },
        )
        .expect("pending should start");
        assert_eq!(pending.remaining_seconds, 5);
    }
}
