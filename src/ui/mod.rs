mod catalog_state;
mod inventory_page;
mod today_page;

use dioxus::prelude::*;
use gloo_timers::future::sleep;
use std::time::Duration;
use web_sys::window;

use crate::domain::models::{AppState, BrewingMatchKind, CatalogOption, ProductLine};
use crate::domain::seed::seed_app_state;
use crate::ui::inventory_page::InventoryPage;
use crate::ui::today_page::TodayPage;
use catalog_state::{
    ArchiveTarget, BrewingPlanCategoryFormInput, BrewingPlanFormInput, CatalogOptionFormInput,
    CoffeeBeanFormInput, FormValidationError, ParameterCatalog, PendingArchive,
    RoastLevelFormInput, RoastMethodFormInput, RoastProfileFormInput, add_matching_attribute,
    begin_pending_archive, cancel_pending_archive, commit_pending_archive, pending_archive_label,
    remove_matching_attribute, suggest_batch_code, upsert_brewing_plan,
    upsert_brewing_plan_category, upsert_coffee_bean, upsert_parameter_option,
    upsert_roast_level_option, upsert_roast_method, upsert_roast_profile,
};

const APP_STYLES: &str = r#"
:root {
  color-scheme: light;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
  background: #f2f3f8;
  color: #171717;
}

.app-shell {
  margin: 0 auto;
  min-height: 100vh;
  max-width: 480px;
  background: #ffffff;
  padding: 14px 12px calc(76px + env(safe-area-inset-bottom));
}

.page-title {
  margin: 0 0 6px;
  font-size: 24px;
  line-height: 1.25;
}

.page-summary {
  margin: 0 0 12px;
  color: #5b5b65;
  font-size: 14px;
  line-height: 1.4;
}

.tabs {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 6px;
  margin-bottom: 10px;
}

.tabs-six {
  grid-template-columns: repeat(3, minmax(0, 1fr));
}

.tab-item {
  border: 1px solid #dadce8;
  border-radius: 8px;
  background: #ffffff;
  color: #373746;
  padding: 8px 6px;
  font-size: 12px;
}

.tab-item-active {
  border-color: #2d5be3;
  background: #eaf0ff;
  color: #1c47c5;
  font-weight: 600;
}

.panel {
  border: 1px solid #e3e5ef;
  border-radius: 8px;
  padding: 10px;
  margin-bottom: 10px;
}

.panel-title {
  margin: 0 0 8px;
  font-size: 15px;
}

.grid {
  display: grid;
  gap: 8px;
}

.field-label {
  display: block;
  margin-bottom: 4px;
  font-size: 12px;
  color: #4b4b58;
}

.text-input,
.select-input,
.number-input,
.textarea-input {
  width: 100%;
  border: 1px solid #ced2e1;
  border-radius: 8px;
  padding: 9px 10px;
  font-size: 14px;
  background: #ffffff;
}

.textarea-input {
  min-height: 72px;
  resize: vertical;
}

.action-row {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.button {
  border: 1px solid #cad1eb;
  border-radius: 8px;
  background: #eef2ff;
  color: #2f4fc2;
  padding: 9px 8px;
  font-size: 13px;
}

.button-secondary {
  background: #ffffff;
  color: #464655;
}

.button-danger {
  border-color: #f0c8c8;
  background: #fff2f2;
  color: #c03333;
}

.list {
  display: grid;
  gap: 8px;
}

.list-item {
  border: 1px solid #ececf3;
  border-radius: 8px;
  padding: 8px;
  background: #fbfbfd;
}

.list-line {
  margin: 0 0 4px;
  font-size: 13px;
  line-height: 1.35;
}

.chip-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.chip {
  border: 1px solid #cfd4e8;
  border-radius: 8px;
  padding: 4px 6px;
  font-size: 12px;
  background: #f5f7ff;
}

.error-text {
  margin: 4px 0 0;
  color: #c43232;
  font-size: 12px;
}

.pending {
  border: 1px solid #f6df9a;
  background: #fff8dd;
  border-radius: 8px;
  padding: 8px;
  margin-bottom: 10px;
}

.pending-text {
  margin: 0 0 8px;
  font-size: 12px;
}

.bottom-nav {
  position: fixed;
  left: 50%;
  bottom: 0;
  transform: translateX(-50%);
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  width: min(480px, 100%);
  padding: 10px 12px calc(10px + env(safe-area-inset-bottom));
  gap: 8px;
  border-top: 1px solid #e6e6ea;
  background: #ffffff;
}

.nav-item {
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: #6b6b70;
  padding: 10px 8px;
  font-size: 14px;
  line-height: 1.2;
}

.nav-item-active {
  background: #eef2ff;
  color: #4338ca;
  font-weight: 600;
}
"#;

#[derive(Clone, Copy, Eq, PartialEq)]
enum AppPage {
    Today,
    Inventory,
    Catalog,
    Settings,
}

impl AppPage {
    const ALL: [Self; 4] = [Self::Today, Self::Inventory, Self::Catalog, Self::Settings];

    const fn label(self) -> &'static str {
        match self {
            Self::Today => "今日",
            Self::Inventory => "入库",
            Self::Catalog => "资料",
            Self::Settings => "设置",
        }
    }

    const fn summary(self) -> &'static str {
        match self {
            Self::Today => "查看今日冲煮安排与批次状态。",
            Self::Inventory => "录入新豆批次与基础信息。",
            Self::Catalog => "维护目录项与冲煮方案资料。",
            Self::Settings => "配置门店偏好和系统参数。",
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum CatalogSection {
    Parameters,
    Beans,
    RoastMethods,
    RoastProfiles,
    PlanCategories,
    BrewingPlans,
}

impl CatalogSection {
    const ALL: [Self; 6] = [
        Self::Parameters,
        Self::Beans,
        Self::RoastMethods,
        Self::RoastProfiles,
        Self::PlanCategories,
        Self::BrewingPlans,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Parameters => "参数目录",
            Self::Beans => "咖啡豆",
            Self::RoastMethods => "烘焙方法",
            Self::RoastProfiles => "烘焙品类",
            Self::PlanCategories => "方案分类",
            Self::BrewingPlans => "冲煮方案",
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ParameterTab {
    BeanVarieties,
    RoastLevels,
    ProcessingMethods,
}

impl ParameterTab {
    const ALL: [Self; 3] = [
        Self::BeanVarieties,
        Self::RoastLevels,
        Self::ProcessingMethods,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::BeanVarieties => "豆种",
            Self::RoastLevels => "烘焙度",
            Self::ProcessingMethods => "处理法",
        }
    }
}

#[derive(Clone, Default)]
struct BeanFormState {
    editing_id: Option<String>,
    name: String,
    variety_id: String,
    processing_method_id: String,
    origin: String,
    notes: String,
}

#[derive(Clone, Default)]
struct RoastMethodFormState {
    editing_id: Option<String>,
    name: String,
    notes: String,
}

#[derive(Clone)]
struct RoastProfileFormState {
    editing_id: Option<String>,
    bean_id: String,
    method_id: String,
    roast_level_id: String,
    product_line: ProductLine,
    batch_code: String,
    batch_code_customized: bool,
}

impl Default for RoastProfileFormState {
    fn default() -> Self {
        Self {
            editing_id: None,
            bean_id: String::new(),
            method_id: String::new(),
            roast_level_id: String::new(),
            product_line: ProductLine::PourOver,
            batch_code: String::new(),
            batch_code_customized: false,
        }
    }
}

#[derive(Clone, Default)]
struct PlanCategoryFormState {
    editing_id: Option<String>,
    name: String,
}

#[derive(Clone)]
struct PlanFormState {
    editing_id: Option<String>,
    category_id: String,
    name: String,
    matching_attributes: Vec<crate::domain::models::BrewingMatchAttribute>,
    pour_stages: String,
    dripper: String,
    grinder_profile_id: String,
    ratio_coffee: String,
    ratio_water: String,
    default_dose_g: String,
    day0_grind_size: String,
    day0_water_temp_c: String,
    day14_grind_size: String,
    day14_water_temp_c: String,
    instructions: String,
    priority: String,
}

impl Default for PlanFormState {
    fn default() -> Self {
        Self {
            editing_id: None,
            category_id: String::new(),
            name: String::new(),
            matching_attributes: Vec::new(),
            pour_stages: "3".to_string(),
            dripper: String::new(),
            grinder_profile_id: String::new(),
            ratio_coffee: "1.0".to_string(),
            ratio_water: "15.0".to_string(),
            default_dose_g: "16.0".to_string(),
            day0_grind_size: "6.0".to_string(),
            day0_water_temp_c: "92.0".to_string(),
            day14_grind_size: "7.0".to_string(),
            day14_water_temp_c: "90.0".to_string(),
            instructions: String::new(),
            priority: "1".to_string(),
        }
    }
}

pub(crate) fn read_store_id() -> String {
    window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item("coffee_erp:store_id").ok().flatten())
        .unwrap_or_else(|| "store-default".to_string())
}

pub(crate) async fn save_app_state(state: &AppState, store_id: &str) -> Result<AppState, String> {
    let base_url = option_env!("PUBLIC_API_BASE_URL").unwrap_or("http://localhost:8787");
    match crate::storage::save_remote_state_wasm(base_url, store_id, state).await {
        Ok(new_state) => {
            let _ = crate::storage::indexed_db::save_cached_state_web(store_id, &new_state).await;
            Ok(new_state)
        }
        Err(crate::storage::SaveRemoteStateError::RevisionConflict(_)) => {
            Err("保存失败：版本冲突，请刷新后重试".to_string())
        }
        Err(e) => Err(format!("保存失败: {:?}", e)),
    }
}

#[component]
pub fn App() -> Element {
    let mut current_page = use_signal(|| AppPage::Today);
    let app_state = use_signal(seed_app_state);
    let pending_archive = use_signal(|| None::<PendingArchive>);
    let store_id = use_signal(|| read_store_id());
    let save_status = use_signal(|| None::<String>);

    use_effect(move || {
        let sid = store_id();
        let mut app_state_signal = app_state;
        spawn(async move {
            let base_url = option_env!("PUBLIC_API_BASE_URL").unwrap_or("http://localhost:8787");
            // 1. 尝试读 IndexedDB 缓存
            if let Ok(Some(cached)) = crate::storage::indexed_db::load_cached_state_web(&sid).await
            {
                app_state_signal.set(cached);
            }
            // 2. 拉远端
            match crate::storage::load_remote_state_wasm(base_url, &sid).await {
                Ok(remote_state) => {
                    app_state_signal.set(remote_state.clone());
                    // 3. 覆盖 IndexedDB 缓存
                    let _ = crate::storage::indexed_db::save_cached_state_web(&sid, &remote_state)
                        .await;
                }
                Err(_) => {}
            }
        });
    });

    rsx! {
        style { "{APP_STYLES}" }
        main { class: "app-shell",
            h1 { class: "page-title", "{current_page().label()}" }
            p { class: "page-summary", "{current_page().summary()}" }
            match current_page() {
                AppPage::Today => rsx! {
                    TodayPage { app_state }
                },
                AppPage::Inventory => rsx! {
                    InventoryPage { app_state, pending_archive, store_id, save_status }
                },
                AppPage::Catalog => rsx! {
                    CatalogPage { app_state, pending_archive, store_id, save_status }
                },
                AppPage::Settings => rsx! {
                    section { class: "panel",
                        p { class: "list-line", "设置页面在后续里程碑推进。" }
                    }
                },
            }
        }
        nav { class: "bottom-nav",
            for page in AppPage::ALL {
                button {
                    class: if current_page() == page { "nav-item nav-item-active" } else { "nav-item" },
                    onclick: move |_| current_page.set(page),
                    "{page.label()}"
                }
            }
        }
    }
}

#[component]
fn CatalogPage(
    app_state: Signal<AppState>,
    pending_archive: Signal<Option<PendingArchive>>,
    store_id: Signal<String>,
    save_status: Signal<Option<String>>,
) -> Element {
    use_context_provider(|| store_id);
    use_context_provider(|| save_status);
    let mut section = use_signal(|| CatalogSection::Parameters);
    let parameter_tab = use_signal(|| ParameterTab::BeanVarieties);
    let parameter_form = use_signal(CatalogOptionFormInput::default);
    let roast_level_form = use_signal(RoastLevelFormInput::default);
    let bean_form = use_signal(BeanFormState::default);
    let method_form = use_signal(RoastMethodFormState::default);
    let profile_form = use_signal(RoastProfileFormState::default);
    let category_form = use_signal(PlanCategoryFormState::default);
    let plan_form = use_signal(PlanFormState::default);
    let plan_attr_kind = use_signal(|| BrewingMatchKind::ProcessingMethod);
    let plan_attr_option_id = use_signal(String::new);
    let mut errors = use_signal(Vec::<FormValidationError>::new);

    let state = app_state();
    let archive_pending = pending_archive();

    let archive_section = match &archive_pending {
        Some(pending) => rsx! {
            section { class: "pending",
                p { class: "pending-text", "待归档：{pending_archive_label(&state, pending)}。{pending.remaining_seconds} 秒后可撤销或立即提交。" }
                div { class: "action-row",
                    button {
                        class: "button button-secondary",
                        onclick: move |_| {
                            let mut slot = pending_archive.write();
                            let _ = cancel_pending_archive(&mut slot);
                        },
                        "撤销"
                    }
                    button {
                        class: "button button-danger",
                        onclick: move |_| {
                            let pending_to_commit = pending_archive();
                            if let Some(pending) = pending_to_commit {
                                let new_state = {
                                    let mut current = app_state.write();
                                    if commit_pending_archive(&mut current, pending).is_ok() {
                                        pending_archive.set(None);
                                        Some(current.clone())
                                    } else {
                                        None
                                    }
                                };
                                if let Some(state) = new_state {
                                    let sid = store_id();
                                    spawn(async move {
                                        save_status.set(Some("保存中...".to_string()));
                                        match save_app_state(&state, &sid).await {
                                            Ok(saved) => {
                                                app_state.set(saved);
                                                save_status.set(Some("保存成功".to_string()));
                                            }
                                            Err(e) => save_status.set(Some(e)),
                                        }
                                        sleep(Duration::from_secs(3)).await;
                                        save_status.set(None);
                                    });
                                }
                            }
                        },
                        "确认归档"
                    }
                }
            }
        },
        None => rsx! {},
    };

    let save_banner = match save_status() {
        Some(msg) => rsx! {
            section { class: "pending",
                p { class: "pending-text", "{msg}" }
            }
        },
        None => rsx! {},
    };

    rsx! {
        {archive_section}
        {save_banner}
        div { class: "tabs tabs-six",
            for item in CatalogSection::ALL {
                button {
                    class: if section() == item { "tab-item tab-item-active" } else { "tab-item" },
                    onclick: move |_| {
                        section.set(item);
                        errors.set(Vec::new());
                    },
                    "{item.label()}"
                }
            }
        }
        match section() {
            CatalogSection::Parameters => rsx! {
                ParametersPanel {
                    app_state,
                    pending_archive,
                    parameter_tab,
                    parameter_form,
                    roast_level_form,
                    errors,
                }
            },
            CatalogSection::Beans => rsx! {
                BeansPanel { app_state, pending_archive, bean_form, errors }
            },
            CatalogSection::RoastMethods => rsx! {
                RoastMethodsPanel { app_state, pending_archive, method_form, errors }
            },
            CatalogSection::RoastProfiles => rsx! {
                RoastProfilesPanel { app_state, pending_archive, profile_form, errors }
            },
            CatalogSection::PlanCategories => rsx! {
                PlanCategoriesPanel { app_state, pending_archive, category_form, errors }
            },
            CatalogSection::BrewingPlans => rsx! {
                BrewingPlansPanel {
                    app_state,
                    pending_archive,
                    plan_form,
                    plan_attr_kind,
                    plan_attr_option_id,
                    errors,
                }
            },
        }
    }
}

#[component]
fn ParametersPanel(
    app_state: Signal<AppState>,
    pending_archive: Signal<Option<PendingArchive>>,
    parameter_tab: Signal<ParameterTab>,
    parameter_form: Signal<CatalogOptionFormInput>,
    roast_level_form: Signal<RoastLevelFormInput>,
    errors: Signal<Vec<FormValidationError>>,
) -> Element {
    let state = app_state();
    let archive_locked = !catalog_state::can_write_catalog(pending_archive().as_ref());
    let store_id: Signal<String> = use_context();
    let mut save_status: Signal<Option<String>> = use_context();

    let option_items: Vec<(String, String, bool)> = match parameter_tab() {
        ParameterTab::BeanVarieties => state
            .coffee_parameters
            .bean_varieties
            .iter()
            .map(|item| (item.id.clone(), item.label.clone(), item.archived))
            .collect(),
        ParameterTab::ProcessingMethods => state
            .coffee_parameters
            .processing_methods
            .iter()
            .map(|item| (item.id.clone(), item.label.clone(), item.archived))
            .collect(),
        ParameterTab::RoastLevels => state
            .coffee_parameters
            .roast_levels
            .iter()
            .map(|item| {
                (
                    item.id.clone(),
                    format!("{} ({})", item.label, item.agtron_range),
                    item.archived,
                )
            })
            .collect(),
    };

    rsx! {
        div { class: "tabs",
            for tab in ParameterTab::ALL {
                button {
                    class: if parameter_tab() == tab { "tab-item tab-item-active" } else { "tab-item" },
                    onclick: move |_| {
                        parameter_tab.set(tab);
                        errors.set(Vec::new());
                    },
                    "{tab.label()}"
                }
            }
        }
        section { class: "panel",
            h2 { class: "panel-title", "目录列表" }
            div { class: "list",
                for (id, label, archived) in option_items {
                    div { class: "list-item",
                        p { class: "list-line", "{label}" }
                        p { class: "list-line", if archived { "状态：已归档" } else { "状态：生效中" } }
                        div { class: "action-row",
                            button {
                                class: "button button-secondary",
                                disabled: archive_locked,
                                onclick: {
                                    let edit_id = id.clone();
                                    let edit_label = label.clone();
                                    move |_| {
                                    errors.set(Vec::new());
                                    match parameter_tab() {
                                        ParameterTab::RoastLevels => {
                                            if let Some(level) = app_state.read().coffee_parameters.roast_levels.iter().find(|item| item.id == edit_id) {
                                                roast_level_form.set(RoastLevelFormInput {
                                                    editing_id: Some(level.id.clone()),
                                                    label: level.label.clone(),
                                                    agtron_range: level.agtron_range.clone(),
                                                });
                                            }
                                        }
                                        _ => {
                                            parameter_form.set(CatalogOptionFormInput {
                                                editing_id: Some(edit_id.clone()),
                                                label: edit_label.split(" (").next().unwrap_or("").to_string(),
                                            });
                                        }
                                    }
                                }
                                },
                                "编辑"
                            }
                            button {
                                class: "button button-danger",
                                disabled: archive_locked,
                                onclick: {
                                    let archive_id = id.clone();
                                    move |_| {
                                    let target = match parameter_tab() {
                                        ParameterTab::BeanVarieties => ArchiveTarget::BeanVariety { id: archive_id.clone() },
                                        ParameterTab::RoastLevels => ArchiveTarget::RoastLevel { id: archive_id.clone() },
                                        ParameterTab::ProcessingMethods => ArchiveTarget::ProcessingMethod { id: archive_id.clone() },
                                    };
                                    let current_pending = pending_archive();
                                    if let Ok(pending) = begin_pending_archive(&current_pending, target) {
                                        pending_archive.set(Some(pending));
                                        let mut app_state_clone = app_state;
                                        let mut pending_archive_clone = pending_archive;
                                        let store_id_clone = store_id;
                                        let mut save_status_clone = save_status;
                                        spawn(async move {
                                            for _ in 0..5 {
                                                sleep(Duration::from_secs(1)).await;
                                                if let Some(ref mut p) = pending_archive_clone.write().as_mut() {
                                                    if p.remaining_seconds > 0 {
                                                        p.remaining_seconds -= 1;
                                                    }
                                                } else {
                                                    break;
                                                }
                                            }
                                            if let Some(p) = pending_archive_clone() {
                                                if p.remaining_seconds == 0 {
                                                    let committed = {
                                                        let mut state = app_state_clone.write();
                                                        commit_pending_archive(&mut state, p).is_ok()
                                                    };
                                                    if committed {
                                                        pending_archive_clone.set(None);
                                                        let saved_state = app_state_clone.read().clone();
                                                        let sid = store_id_clone();
                                                        spawn(async move {
                                                            save_status_clone.set(Some("保存中...".to_string()));
                                                            match save_app_state(&saved_state, &sid).await {
                                                                Ok(new_state) => {
                                                                    app_state_clone.set(new_state);
                                                                    save_status_clone.set(Some("保存成功".to_string()));
                                                                }
                                                                Err(e) => save_status_clone.set(Some(e)),
                                                            }
                                                            sleep(Duration::from_secs(3)).await;
                                                            save_status_clone.set(None);
                                                        });
                                                    }
                                                }
                                            }
                                        });
                                    }
                                }
                                },
                                "归档"
                            }
                        }
                    }
                }
            }
            match parameter_tab() {
                ParameterTab::RoastLevels => rsx! {
                    div { class: "grid",
                        div {
                            label { class: "field-label", "标签名称" }
                            input {
                                class: "text-input",
                                value: "{roast_level_form.read().label}",
                                oninput: move |event| {
                                    roast_level_form.write().label = event.value();
                                },
                            }
                            FieldError { errors, field: "label" }
                        }
                        div {
                            label { class: "field-label", "Agtron 范围" }
                            input {
                                class: "text-input",
                                value: "{roast_level_form.read().agtron_range}",
                                oninput: move |event| {
                                    roast_level_form.write().agtron_range = event.value();
                                },
                            }
                            FieldError { errors, field: "agtron_range" }
                        }
                        div { class: "action-row",
                            button {
                                class: "button",
                                disabled: archive_locked,
                                onclick: move |_| {
                                    let input = roast_level_form.read().clone();
                                    let result = { let mut state = app_state.write(); upsert_roast_level_option(&mut state, &input) };
                                    match result {
                                        Ok(_) => {
                                            errors.set(Vec::new());
                                            roast_level_form.set(RoastLevelFormInput::default());
                                            let state = app_state.read().clone();
                                            let sid = store_id();
                                            spawn(async move {
                                                save_status.set(Some("保存中...".to_string()));
                                                match save_app_state(&state, &sid).await {
                                                    Ok(new_state) => {
                                                        app_state.set(new_state);
                                                        save_status.set(Some("保存成功".to_string()));
                                                    }
                                                    Err(e) => save_status.set(Some(e)),
                                                }
                                                sleep(Duration::from_secs(3)).await;
                                                save_status.set(None);
                                            });
                                        }
                                        Err(validation_errors) => errors.set(validation_errors),
                                    }
                                },
                                "保存"
                            }
                            button {
                                class: "button button-secondary",
                                disabled: archive_locked,
                                onclick: move |_| {
                                    roast_level_form.set(RoastLevelFormInput::default());
                                    errors.set(Vec::new());
                                },
                                "清空"
                            }
                        }
                    }
                },
                _ => rsx! {
                    div { class: "grid",
                        div {
                            label { class: "field-label", "标签名称" }
                            input {
                                class: "text-input",
                                value: "{parameter_form.read().label}",
                                oninput: move |event| {
                                    parameter_form.write().label = event.value();
                                },
                            }
                            FieldError { errors, field: "label" }
                        }
                        div { class: "action-row",
                            button {
                                class: "button",
                                disabled: archive_locked,
                                onclick: move |_| {
                                    let input = parameter_form.read().clone();
                                    let catalog = if parameter_tab() == ParameterTab::BeanVarieties {
                                        ParameterCatalog::BeanVariety
                                    } else {
                                        ParameterCatalog::ProcessingMethod
                                    };
                                    let result = { let mut state = app_state.write(); upsert_parameter_option(&mut state, catalog, &input) };
                                    match result {
                                        Ok(_) => {
                                            errors.set(Vec::new());
                                            parameter_form.set(CatalogOptionFormInput::default());
                                            let state = app_state.read().clone();
                                            let sid = store_id();
                                            spawn(async move {
                                                save_status.set(Some("保存中...".to_string()));
                                                match save_app_state(&state, &sid).await {
                                                    Ok(new_state) => {
                                                        app_state.set(new_state);
                                                        save_status.set(Some("保存成功".to_string()));
                                                    }
                                                    Err(e) => save_status.set(Some(e)),
                                                }
                                                sleep(Duration::from_secs(3)).await;
                                                save_status.set(None);
                                            });
                                        }
                                        Err(validation_errors) => errors.set(validation_errors),
                                    }
                                },
                                "保存"
                            }
                            button {
                                class: "button button-secondary",
                                disabled: archive_locked,
                                onclick: move |_| {
                                    parameter_form.set(CatalogOptionFormInput::default());
                                    errors.set(Vec::new());
                                },
                                "清空"
                            }
                        }
                    }
                },
            }
        }
    }
}

#[component]
fn BeansPanel(
    app_state: Signal<AppState>,
    pending_archive: Signal<Option<PendingArchive>>,
    bean_form: Signal<BeanFormState>,
    errors: Signal<Vec<FormValidationError>>,
) -> Element {
    let state = app_state();
    let archive_locked = !catalog_state::can_write_catalog(pending_archive().as_ref());
    let store_id: Signal<String> = use_context();
    let mut save_status: Signal<Option<String>> = use_context();

    rsx! {
        section { class: "panel",
            h2 { class: "panel-title", "咖啡豆列表" }
            div { class: "list",
                for bean in state.beans.iter() {
                    div { class: "list-item",
                        p { class: "list-line", "{bean.name}" }
                        p { class: "list-line", "产地：{bean.origin.clone().unwrap_or_default()}" }
                        p { class: "list-line", if bean.archived { "状态：已归档" } else { "状态：生效中" } }
                        div { class: "action-row",
                            button {
                                class: "button button-secondary",
                                disabled: archive_locked,
                                onclick: {
                                    let bean_for_edit = bean.clone();
                                    move |_| {
                                    bean_form.set(BeanFormState {
                                        editing_id: Some(bean_for_edit.id.clone()),
                                        name: bean_for_edit.name.clone(),
                                        variety_id: bean_for_edit.variety_id.clone().unwrap_or_default(),
                                        processing_method_id: bean_for_edit.processing_method_id.clone().unwrap_or_default(),
                                        origin: bean_for_edit.origin.clone().unwrap_or_default(),
                                        notes: bean_for_edit.notes.clone().unwrap_or_default(),
                                    });
                                    errors.set(Vec::new());
                                }
                                },
                                "编辑"
                            }
                            button {
                                class: "button button-danger",
                                disabled: archive_locked,
                                onclick: {
                                    let bean_for_archive = bean.clone();
                                    move |_| {
                                    let current_pending = pending_archive();
                                    if let Ok(pending) = begin_pending_archive(
                                        &current_pending,
                                        ArchiveTarget::CoffeeBean { id: bean_for_archive.id.clone() }
                                    ) {
                                        pending_archive.set(Some(pending));
                                    }
                                }
                                },
                                "归档"
                            }
                        }
                    }
                }
            }
        }
        section { class: "panel",
            h2 { class: "panel-title", "咖啡豆表单" }
            div { class: "grid",
                div {
                    label { class: "field-label", "名称" }
                    input {
                        class: "text-input",
                        value: "{bean_form.read().name}",
                        oninput: move |event| bean_form.write().name = event.value(),
                    }
                    FieldError { errors, field: "name" }
                }
                div {
                    label { class: "field-label", "豆种" }
                    select {
                        class: "select-input",
                        value: "{bean_form.read().variety_id}",
                        onchange: move |event| bean_form.write().variety_id = event.value(),
                        option { value: "", "未指定" }
                        for option in state.coffee_parameters.bean_varieties.iter().filter(|item| !item.archived) {
                            option { value: "{option.id}", "{option.label}" }
                        }
                    }
                    FieldError { errors, field: "variety_id" }
                }
                div {
                    label { class: "field-label", "处理法" }
                    select {
                        class: "select-input",
                        value: "{bean_form.read().processing_method_id}",
                        onchange: move |event| bean_form.write().processing_method_id = event.value(),
                        option { value: "", "未指定" }
                        for option in state.coffee_parameters.processing_methods.iter().filter(|item| !item.archived) {
                            option { value: "{option.id}", "{option.label}" }
                        }
                    }
                    FieldError { errors, field: "processing_method_id" }
                }
                div {
                    label { class: "field-label", "产地" }
                    input {
                        class: "text-input",
                        value: "{bean_form.read().origin}",
                        oninput: move |event| bean_form.write().origin = event.value(),
                    }
                }
                div {
                    label { class: "field-label", "备注" }
                    textarea {
                        class: "textarea-input",
                        value: "{bean_form.read().notes}",
                        oninput: move |event| bean_form.write().notes = event.value(),
                    }
                }
                div { class: "action-row",
                    button {
                        class: "button",
                        disabled: archive_locked,
                        onclick: move |_| {
                            let input = bean_form.read().clone();
                            let payload = CoffeeBeanFormInput {
                                editing_id: input.editing_id,
                                name: input.name,
                                variety_id: blank_to_none(input.variety_id),
                                processing_method_id: blank_to_none(input.processing_method_id),
                                origin: input.origin,
                                notes: input.notes,
                            };
                            let result = { let mut state = app_state.write(); upsert_coffee_bean(&mut state, &payload) };
                            match result {
                                Ok(_) => {
                                    bean_form.set(BeanFormState::default());
                                    errors.set(Vec::new());
                                    let state = app_state.read().clone();
                                    let sid = store_id();
                                    spawn(async move {
                                        save_status.set(Some("保存中...".to_string()));
                                        match save_app_state(&state, &sid).await {
                                            Ok(new_state) => {
                                                app_state.set(new_state);
                                                save_status.set(Some("保存成功".to_string()));
                                            }
                                            Err(e) => save_status.set(Some(e)),
                                        }
                                        sleep(Duration::from_secs(3)).await;
                                        save_status.set(None);
                                    });
                                }
                                Err(validation_errors) => errors.set(validation_errors),
                            }
                        },
                        "保存"
                    }
                    button {
                        class: "button button-secondary",
                        disabled: archive_locked,
                        onclick: move |_| {
                            bean_form.set(BeanFormState::default());
                            errors.set(Vec::new());
                        },
                        "清空"
                    }
                }
            }
        }
    }
}

#[component]
fn RoastMethodsPanel(
    app_state: Signal<AppState>,
    pending_archive: Signal<Option<PendingArchive>>,
    method_form: Signal<RoastMethodFormState>,
    errors: Signal<Vec<FormValidationError>>,
) -> Element {
    let state = app_state();
    let archive_locked = !catalog_state::can_write_catalog(pending_archive().as_ref());
    let store_id: Signal<String> = use_context();
    let mut save_status: Signal<Option<String>> = use_context();

    rsx! {
        section { class: "panel",
            h2 { class: "panel-title", "烘焙方法列表" }
            div { class: "list",
                for item in state.roast_methods.iter() {
                    div { class: "list-item",
                        p { class: "list-line", "{item.name}" }
                        p { class: "list-line", if item.archived { "状态：已归档" } else { "状态：生效中" } }
                        div { class: "action-row",
                            button {
                                class: "button button-secondary",
                                disabled: archive_locked,
                                onclick: {
                                    let method_for_edit = item.clone();
                                    move |_| {
                                    method_form.set(RoastMethodFormState {
                                        editing_id: Some(method_for_edit.id.clone()),
                                        name: method_for_edit.name.clone(),
                                        notes: method_for_edit.notes.clone().unwrap_or_default(),
                                    });
                                    errors.set(Vec::new());
                                }
                                },
                                "编辑"
                            }
                            button {
                                class: "button button-danger",
                                disabled: archive_locked,
                                onclick: {
                                    let method_for_archive = item.clone();
                                    move |_| {
                                    let current_pending = pending_archive();
                                    if let Ok(pending) = begin_pending_archive(
                                        &current_pending,
                                        ArchiveTarget::RoastMethod { id: method_for_archive.id.clone() },
                                    ) {
                                        pending_archive.set(Some(pending));
                                    }
                                }
                                },
                                "归档"
                            }
                        }
                    }
                }
            }
        }
        section { class: "panel",
            h2 { class: "panel-title", "烘焙方法表单" }
            div { class: "grid",
                div {
                    label { class: "field-label", "方法名称" }
                    input {
                        class: "text-input",
                        value: "{method_form.read().name}",
                        oninput: move |event| method_form.write().name = event.value(),
                    }
                    FieldError { errors, field: "name" }
                }
                div {
                    label { class: "field-label", "备注" }
                    textarea {
                        class: "textarea-input",
                        value: "{method_form.read().notes}",
                        oninput: move |event| method_form.write().notes = event.value(),
                    }
                }
                div { class: "action-row",
                    button {
                        class: "button",
                        disabled: archive_locked,
                        onclick: move |_| {
                            let input = method_form.read().clone();
                            let payload = RoastMethodFormInput {
                                editing_id: input.editing_id,
                                name: input.name,
                                notes: input.notes,
                            };
                            let result = { let mut state = app_state.write(); upsert_roast_method(&mut state, &payload) };
                            match result {
                                Ok(_) => {
                                    method_form.set(RoastMethodFormState::default());
                                    errors.set(Vec::new());
                                    let state = app_state.read().clone();
                                    let sid = store_id();
                                    spawn(async move {
                                        save_status.set(Some("保存中...".to_string()));
                                        match save_app_state(&state, &sid).await {
                                            Ok(new_state) => {
                                                app_state.set(new_state);
                                                save_status.set(Some("保存成功".to_string()));
                                            }
                                            Err(e) => save_status.set(Some(e)),
                                        }
                                        sleep(Duration::from_secs(3)).await;
                                        save_status.set(None);
                                    });
                                }
                                Err(validation_errors) => errors.set(validation_errors),
                            }
                        },
                        "保存"
                    }
                    button {
                        class: "button button-secondary",
                        disabled: archive_locked,
                        onclick: move |_| {
                            method_form.set(RoastMethodFormState::default());
                            errors.set(Vec::new());
                        },
                        "清空"
                    }
                }
            }
        }
    }
}

#[component]
fn RoastProfilesPanel(
    app_state: Signal<AppState>,
    pending_archive: Signal<Option<PendingArchive>>,
    profile_form: Signal<RoastProfileFormState>,
    errors: Signal<Vec<FormValidationError>>,
) -> Element {
    let state = app_state();
    let archive_locked = !catalog_state::can_write_catalog(pending_archive().as_ref());
    let store_id: Signal<String> = use_context();
    let mut save_status: Signal<Option<String>> = use_context();

    rsx! {
        section { class: "panel",
            h2 { class: "panel-title", "烘焙品类列表" }
            div { class: "list",
                for item in state.roast_profiles.iter() {
                    div { class: "list-item",
                        p { class: "list-line", "{item.display_name}" }
                        p { class: "list-line", "batch_code：{item.batch_code}" }
                        p { class: "list-line", if item.archived { "状态：已归档" } else { "状态：生效中" } }
                        div { class: "action-row",
                            button {
                                class: "button button-secondary",
                                disabled: archive_locked,
                                onclick: {
                                    let profile_for_edit = item.clone();
                                    move |_| {
                                    profile_form.set(RoastProfileFormState {
                                        editing_id: Some(profile_for_edit.id.clone()),
                                        bean_id: profile_for_edit.bean_id.clone(),
                                        method_id: profile_for_edit.method_id.clone(),
                                        roast_level_id: profile_for_edit.roast_level_id.clone().unwrap_or_default(),
                                        product_line: profile_for_edit.product_line,
                                        batch_code: profile_for_edit.batch_code.clone(),
                                        batch_code_customized: true,
                                    });
                                    errors.set(Vec::new());
                                }
                                },
                                "编辑"
                            }
                            button {
                                class: "button button-danger",
                                disabled: archive_locked,
                                onclick: {
                                    let profile_for_archive = item.clone();
                                    move |_| {
                                    let current_pending = pending_archive();
                                    if let Ok(pending) = begin_pending_archive(
                                        &current_pending,
                                        ArchiveTarget::RoastProfile { id: profile_for_archive.id.clone() },
                                    ) {
                                        pending_archive.set(Some(pending));
                                    }
                                }
                                },
                                "归档"
                            }
                        }
                    }
                }
            }
        }
        section { class: "panel",
            h2 { class: "panel-title", "烘焙品类表单" }
            div { class: "grid",
                div {
                    label { class: "field-label", "咖啡豆" }
                    select {
                        class: "select-input",
                        value: "{profile_form.read().bean_id}",
                        onchange: move |event| {
                            profile_form.write().bean_id = event.value();
                            if !profile_form.read().batch_code_customized {
                                refresh_profile_batch_code(app_state, profile_form);
                            }
                        },
                        option { value: "", "请选择" }
                        for bean in state.beans.iter().filter(|item| !item.archived) {
                            option { value: "{bean.id}", "{bean.name}" }
                        }
                    }
                    FieldError { errors, field: "bean_id" }
                }
                div {
                    label { class: "field-label", "烘焙方法" }
                    select {
                        class: "select-input",
                        value: "{profile_form.read().method_id}",
                        onchange: move |event| {
                            profile_form.write().method_id = event.value();
                            if !profile_form.read().batch_code_customized {
                                refresh_profile_batch_code(app_state, profile_form);
                            }
                        },
                        option { value: "", "请选择" }
                        for method in state.roast_methods.iter().filter(|item| !item.archived) {
                            option { value: "{method.id}", "{method.name}" }
                        }
                    }
                    FieldError { errors, field: "method_id" }
                }
                div {
                    label { class: "field-label", "烘焙度" }
                    select {
                        class: "select-input",
                        value: "{profile_form.read().roast_level_id}",
                        onchange: move |event| {
                            profile_form.write().roast_level_id = event.value();
                        },
                        option { value: "", "未指定" }
                        for level in state.coffee_parameters.roast_levels.iter().filter(|item| !item.archived) {
                            option { value: "{level.id}", "{level.label}" }
                        }
                    }
                    FieldError { errors, field: "roast_level_id" }
                }
                div {
                    label { class: "field-label", "产品线" }
                    select {
                        class: "select-input",
                        value: if profile_form.read().product_line == ProductLine::PourOver { "pour_over" } else { "espresso" },
                        onchange: move |event| {
                            profile_form.write().product_line = if event.value() == "espresso" {
                                ProductLine::Espresso
                            } else {
                                ProductLine::PourOver
                            };
                            if !profile_form.read().batch_code_customized {
                                refresh_profile_batch_code(app_state, profile_form);
                            }
                        },
                        option { value: "pour_over", "手冲" }
                        option { value: "espresso", "意式" }
                    }
                }
                div {
                    label { class: "field-label", "batch_code" }
                    input {
                        class: "text-input",
                        value: "{profile_form.read().batch_code}",
                        oninput: move |event| {
                            profile_form.write().batch_code = event.value();
                            profile_form.write().batch_code_customized = true;
                        },
                    }
                    FieldError { errors, field: "batch_code" }
                }
                div { class: "action-row",
                    button {
                        class: "button button-secondary",
                        disabled: archive_locked,
                        onclick: move |_| {
                            refresh_profile_batch_code(app_state, profile_form);
                            profile_form.write().batch_code_customized = false;
                        },
                        "自动建议"
                    }
                    button {
                        class: "button",
                        disabled: archive_locked,
                        onclick: move |_| {
                            let input = profile_form.read().clone();
                            let payload = RoastProfileFormInput {
                                editing_id: input.editing_id,
                                bean_id: input.bean_id,
                                method_id: input.method_id,
                                roast_level_id: blank_to_none(input.roast_level_id),
                                product_line: input.product_line,
                                batch_code: input.batch_code,
                            };
                            let result = { let mut state = app_state.write(); upsert_roast_profile(&mut state, &payload) };
                            match result {
                                Ok(_) => {
                                    profile_form.set(RoastProfileFormState::default());
                                    errors.set(Vec::new());
                                    let state = app_state.read().clone();
                                    let sid = store_id();
                                    spawn(async move {
                                        save_status.set(Some("保存中...".to_string()));
                                        match save_app_state(&state, &sid).await {
                                            Ok(new_state) => {
                                                app_state.set(new_state);
                                                save_status.set(Some("保存成功".to_string()));
                                            }
                                            Err(e) => save_status.set(Some(e)),
                                        }
                                        sleep(Duration::from_secs(3)).await;
                                        save_status.set(None);
                                    });
                                }
                                Err(validation_errors) => errors.set(validation_errors),
                            }
                        },
                        "保存"
                    }
                }
            }
        }
    }
}

#[component]
fn PlanCategoriesPanel(
    app_state: Signal<AppState>,
    pending_archive: Signal<Option<PendingArchive>>,
    category_form: Signal<PlanCategoryFormState>,
    errors: Signal<Vec<FormValidationError>>,
) -> Element {
    let state = app_state();
    let archive_locked = !catalog_state::can_write_catalog(pending_archive().as_ref());
    let store_id: Signal<String> = use_context();
    let mut save_status: Signal<Option<String>> = use_context();

    rsx! {
        section { class: "panel",
            h2 { class: "panel-title", "冲煮方案分类" }
            div { class: "list",
                for item in state.brewing_plan_categories.iter() {
                    div { class: "list-item",
                        p { class: "list-line", "{item.name}" }
                        p { class: "list-line", "排序：{item.sort_order}" }
                        p { class: "list-line", if item.archived { "状态：已归档" } else { "状态：生效中" } }
                        div { class: "action-row",
                            button {
                                class: "button button-secondary",
                                disabled: archive_locked,
                                onclick: {
                                    let category_for_edit = item.clone();
                                    move |_| {
                                    category_form.set(PlanCategoryFormState {
                                        editing_id: Some(category_for_edit.id.clone()),
                                        name: category_for_edit.name.clone(),
                                    });
                                    errors.set(Vec::new());
                                }
                                },
                                "编辑"
                            }
                            button {
                                class: "button button-danger",
                                disabled: archive_locked,
                                onclick: {
                                    let category_for_archive = item.clone();
                                    move |_| {
                                    let current_pending = pending_archive();
                                    if let Ok(pending) = begin_pending_archive(
                                        &current_pending,
                                        ArchiveTarget::BrewingPlanCategory { id: category_for_archive.id.clone() },
                                    ) {
                                        pending_archive.set(Some(pending));
                                    }
                                }
                                },
                                "归档"
                            }
                        }
                    }
                }
            }
        }
        section { class: "panel",
            h2 { class: "panel-title", "分类表单" }
            div { class: "grid",
                div {
                    label { class: "field-label", "分类名称" }
                    input {
                        class: "text-input",
                        value: "{category_form.read().name}",
                        oninput: move |event| category_form.write().name = event.value(),
                    }
                    FieldError { errors, field: "name" }
                }
                div { class: "action-row",
                    button {
                        class: "button",
                        disabled: archive_locked,
                        onclick: move |_| {
                            let payload = BrewingPlanCategoryFormInput {
                                editing_id: category_form.read().editing_id.clone(),
                                name: category_form.read().name.clone(),
                            };
                            let result = { let mut state = app_state.write(); upsert_brewing_plan_category(&mut state, &payload) };
                            match result {
                                Ok(_) => {
                                    category_form.set(PlanCategoryFormState::default());
                                    errors.set(Vec::new());
                                    let state = app_state.read().clone();
                                    let sid = store_id();
                                    spawn(async move {
                                        save_status.set(Some("保存中...".to_string()));
                                        match save_app_state(&state, &sid).await {
                                            Ok(new_state) => {
                                                app_state.set(new_state);
                                                save_status.set(Some("保存成功".to_string()));
                                            }
                                            Err(e) => save_status.set(Some(e)),
                                        }
                                        sleep(Duration::from_secs(3)).await;
                                        save_status.set(None);
                                    });
                                }
                                Err(validation_errors) => errors.set(validation_errors),
                            }
                        },
                        "保存"
                    }
                    button {
                        class: "button button-secondary",
                        disabled: archive_locked,
                        onclick: move |_| {
                            category_form.set(PlanCategoryFormState::default());
                            errors.set(Vec::new());
                        },
                        "清空"
                    }
                }
            }
        }
    }
}

#[component]
fn BrewingPlansPanel(
    app_state: Signal<AppState>,
    pending_archive: Signal<Option<PendingArchive>>,
    plan_form: Signal<PlanFormState>,
    plan_attr_kind: Signal<BrewingMatchKind>,
    plan_attr_option_id: Signal<String>,
    errors: Signal<Vec<FormValidationError>>,
) -> Element {
    let state = app_state();
    let archive_locked = !catalog_state::can_write_catalog(pending_archive().as_ref());
    let store_id: Signal<String> = use_context();
    let mut save_status: Signal<Option<String>> = use_context();
    let category_id = plan_form.read().category_id.clone();

    let grinders = state
        .grinder_profiles
        .iter()
        .filter(|item| !item.archived)
        .collect::<Vec<_>>();
    let selected_category_plans = state
        .brewing_plan_categories
        .iter()
        .find(|item| item.id == category_id.as_str())
        .map(|item| item.plans.as_slice())
        .unwrap_or_default();
    let matching_options = options_for_match_kind(&state, plan_attr_kind());

    rsx! {
        section { class: "panel",
            h2 { class: "panel-title", "冲煮方案列表" }
            div {
                label { class: "field-label", "筛选分类" }
                select {
                    class: "select-input",
                    value: "{plan_form.read().category_id}",
                    onchange: move |event| plan_form.write().category_id = event.value(),
                    option { value: "", "请选择分类" }
                    for category in state.brewing_plan_categories.iter().filter(|item| !item.archived) {
                        option { value: "{category.id}", "{category.name}" }
                    }
                }
            }
            div { class: "list",
                for item in selected_category_plans {
                    div { class: "list-item",
                        p { class: "list-line", "{item.name}" }
                        p { class: "list-line", "段数：{item.parameters.pour_stages} / 滤杯：{item.parameters.dripper}" }
                        p { class: "list-line", if item.archived { "状态：已归档" } else { "状态：生效中" } }
                        div { class: "action-row",
                            button {
                                class: "button button-secondary",
                                disabled: archive_locked,
                                onclick: {
                                    let selected_category_id = category_id.clone();
                                    let item = item.clone();
                                    move |_| {
                                    plan_form.set(PlanFormState {
                                        editing_id: Some(item.id.clone()),
                                        category_id: selected_category_id.clone(),
                                        name: item.name.clone(),
                                        matching_attributes: item.matching_attributes.clone(),
                                        pour_stages: item.parameters.pour_stages.to_string(),
                                        dripper: item.parameters.dripper.clone(),
                                        grinder_profile_id: item.parameters.grinder_profile_id.clone().unwrap_or_default(),
                                        ratio_coffee: item.parameters.ratio.coffee.to_string(),
                                        ratio_water: item.parameters.ratio.water.to_string(),
                                        default_dose_g: item.parameters.default_dose_g.to_string(),
                                        day0_grind_size: item.age_fitting.day0.grind_size.to_string(),
                                        day0_water_temp_c: item.age_fitting.day0.water_temp_c.to_string(),
                                        day14_grind_size: item.age_fitting.day14.grind_size.to_string(),
                                        day14_water_temp_c: item.age_fitting.day14.water_temp_c.to_string(),
                                        instructions: item.instructions.clone().unwrap_or_default(),
                                        priority: item.priority.to_string(),
                                    });
                                    errors.set(Vec::new());
                                }
                                },
                                "编辑"
                            }
                            button {
                                class: "button button-danger",
                                disabled: archive_locked,
                                onclick: {
                                    let selected_category_id = category_id.clone();
                                    let item = item.clone();
                                    move |_| {
                                    let current_pending = pending_archive();
                                    if let Ok(pending) = begin_pending_archive(
                                        &current_pending,
                                        ArchiveTarget::BrewingPlan {
                                            category_id: selected_category_id.clone(),
                                            id: item.id.clone(),
                                        },
                                    ) {
                                        pending_archive.set(Some(pending));
                                    }
                                }
                                },
                                "归档"
                            }
                        }
                    }
                }
            }
        }
        section { class: "panel",
            h2 { class: "panel-title", "冲煮方案表单" }
            div { class: "grid",
                div {
                    label { class: "field-label", "分类" }
                    select {
                        class: "select-input",
                        value: "{plan_form.read().category_id}",
                        onchange: move |event| plan_form.write().category_id = event.value(),
                        option { value: "", "请选择分类" }
                        for category in state.brewing_plan_categories.iter().filter(|item| !item.archived) {
                            option { value: "{category.id}", "{category.name}" }
                        }
                    }
                    FieldError { errors, field: "category_id" }
                }
                div {
                    label { class: "field-label", "方案名称" }
                    input {
                        class: "text-input",
                        value: "{plan_form.read().name}",
                        oninput: move |event| plan_form.write().name = event.value(),
                    }
                    FieldError { errors, field: "name" }
                }
                div {
                    label { class: "field-label", "匹配属性" }
                    div { class: "chip-row",
                        for (index, attr) in plan_form.read().matching_attributes.iter().enumerate() {
                            button {
                                class: "chip",
                                disabled: archive_locked,
                                onclick: move |_| {
                                    let _ = remove_matching_attribute(&mut plan_form.write().matching_attributes, index);
                                },
                                "{match_kind_label(attr.kind)}：{option_label_by_kind(&state, attr.kind, &attr.option_id)} x"
                            }
                        }
                    }
                    div { class: "action-row",
                        select {
                            class: "select-input",
                            value: match_kind_value(plan_attr_kind()),
                            onchange: move |event| {
                                plan_attr_kind.set(parse_match_kind(event.value().as_str()));
                                plan_attr_option_id.set(String::new());
                            },
                            option { value: "processing", "处理法" }
                            option { value: "variety", "豆种" }
                            option { value: "roast_level", "烘焙度" }
                        }
                        select {
                            class: "select-input",
                            value: "{plan_attr_option_id.read()}",
                            onchange: move |event| plan_attr_option_id.set(event.value()),
                            option { value: "", "选择属性" }
                            for option in matching_options.iter() {
                                option { value: "{option.id}", "{option.label}" }
                            }
                        }
                    }
                    button {
                        class: "button button-secondary",
                        disabled: archive_locked,
                        onclick: move |_| {
                            let added = add_matching_attribute(
                                &mut plan_form.write().matching_attributes,
                                plan_attr_kind(),
                                &plan_attr_option_id.read(),
                            );
                            if added {
                                errors.set(Vec::new());
                            }
                        },
                        "添加属性"
                    }
                    FieldError { errors, field: "matching_attributes" }
                }
                FieldSetNumber { label: "注水段数", value: plan_form.read().pour_stages.clone(), onchange: move |value| plan_form.write().pour_stages = value, errors, field: "pour_stages" }
                FieldSetText { label: "滤杯", value: plan_form.read().dripper.clone(), onchange: move |value| plan_form.write().dripper = value, errors, field: "dripper" }
                div {
                    label { class: "field-label", "磨豆机" }
                    select {
                        class: "select-input",
                        value: "{plan_form.read().grinder_profile_id}",
                        onchange: move |event| plan_form.write().grinder_profile_id = event.value(),
                        option { value: "", "未指定" }
                        for grinder in grinders.iter() {
                            option { value: "{grinder.id}", "{grinder.name}" }
                        }
                    }
                }
                FieldSetText { label: "比例 coffee", value: plan_form.read().ratio_coffee.clone(), onchange: move |value| plan_form.write().ratio_coffee = value, errors, field: "ratio" }
                FieldSetText { label: "比例 water", value: plan_form.read().ratio_water.clone(), onchange: move |value| plan_form.write().ratio_water = value, errors, field: "ratio" }
                FieldSetText { label: "默认粉量(g)", value: plan_form.read().default_dose_g.clone(), onchange: move |value| plan_form.write().default_dose_g = value, errors, field: "default_dose_g" }
                FieldSetText { label: "Day0 研磨度", value: plan_form.read().day0_grind_size.clone(), onchange: move |value| plan_form.write().day0_grind_size = value, errors, field: "age_fitting" }
                FieldSetText { label: "Day0 水温", value: plan_form.read().day0_water_temp_c.clone(), onchange: move |value| plan_form.write().day0_water_temp_c = value, errors, field: "age_fitting" }
                FieldSetText { label: "Day14 研磨度", value: plan_form.read().day14_grind_size.clone(), onchange: move |value| plan_form.write().day14_grind_size = value, errors, field: "age_fitting" }
                FieldSetText { label: "Day14 水温", value: plan_form.read().day14_water_temp_c.clone(), onchange: move |value| plan_form.write().day14_water_temp_c = value, errors, field: "age_fitting" }
                FieldSetNumber { label: "优先级", value: plan_form.read().priority.clone(), onchange: move |value| plan_form.write().priority = value, errors, field: "priority" }
                div {
                    label { class: "field-label", "说明文字" }
                    textarea {
                        class: "textarea-input",
                        value: "{plan_form.read().instructions}",
                        oninput: move |event| plan_form.write().instructions = event.value(),
                    }
                }
                div { class: "action-row",
                    button {
                        class: "button",
                        disabled: archive_locked,
                        onclick: move |_| {
                            let input_form = plan_form.read().clone();
                            match parse_plan_form_to_input(&input_form) {
                                Ok(payload) => {
                                    let result = { let mut state = app_state.write(); upsert_brewing_plan(&mut state, &payload) };
                                    match result {
                                    Ok(_) => {
                                        plan_form.set(PlanFormState::default());
                                        errors.set(Vec::new());
                                        let state = app_state.read().clone();
                                        let sid = store_id();
                                        spawn(async move {
                                            save_status.set(Some("保存中...".to_string()));
                                            match save_app_state(&state, &sid).await {
                                                Ok(new_state) => {
                                                    app_state.set(new_state);
                                                    save_status.set(Some("保存成功".to_string()));
                                                }
                                                Err(e) => save_status.set(Some(e)),
                                            }
                                            sleep(Duration::from_secs(3)).await;
                                            save_status.set(None);
                                        });
                                    }
                                    Err(validation_errors) => errors.set(validation_errors),
                                    }
                                },
                                Err(parse_errors) => errors.set(parse_errors),
                            }
                        },
                        "保存"
                    }
                    button {
                        class: "button button-secondary",
                        disabled: archive_locked,
                        onclick: move |_| {
                            plan_form.set(PlanFormState::default());
                            errors.set(Vec::new());
                        },
                        "清空"
                    }
                }
            }
        }
    }
}

#[component]
fn FieldError(errors: Signal<Vec<FormValidationError>>, field: &'static str) -> Element {
    let error_message = errors
        .read()
        .iter()
        .find(|error| error.field == field || error.field.starts_with(field))
        .map(|error| error.message.clone());
    match error_message {
        Some(message) => rsx! { p { class: "error-text", "{message}" } },
        None => rsx! {},
    }
}

#[component]
fn FieldSetText(
    label: &'static str,
    value: String,
    onchange: EventHandler<String>,
    errors: Signal<Vec<FormValidationError>>,
    field: &'static str,
) -> Element {
    rsx! {
        div {
            label { class: "field-label", "{label}" }
            input {
                class: "text-input",
                value: "{value}",
                oninput: move |event| onchange.call(event.value()),
            }
            FieldError { errors, field }
        }
    }
}

#[component]
fn FieldSetNumber(
    label: &'static str,
    value: String,
    onchange: EventHandler<String>,
    errors: Signal<Vec<FormValidationError>>,
    field: &'static str,
) -> Element {
    rsx! {
        div {
            label { class: "field-label", "{label}" }
            input {
                class: "number-input",
                value: "{value}",
                oninput: move |event| onchange.call(event.value()),
            }
            FieldError { errors, field }
        }
    }
}

fn blank_to_none(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn refresh_profile_batch_code(
    app_state: Signal<AppState>,
    mut profile_form: Signal<RoastProfileFormState>,
) {
    let state = app_state();
    let form_snapshot = profile_form();
    let bean_name = state
        .beans
        .iter()
        .find(|item| item.id == form_snapshot.bean_id)
        .map(|item| item.name.as_str())
        .unwrap_or("");
    let method_name = state
        .roast_methods
        .iter()
        .find(|item| item.id == form_snapshot.method_id)
        .map(|item| item.name.as_str())
        .unwrap_or("");
    let code = suggest_batch_code(bean_name, method_name, form_snapshot.product_line);
    profile_form.write().batch_code = code;
}

fn parse_plan_form_to_input(
    form: &PlanFormState,
) -> Result<BrewingPlanFormInput, Vec<FormValidationError>> {
    let mut errors = Vec::new();
    let pour_stages = parse_u8_field("pour_stages", &form.pour_stages, &mut errors);
    let ratio_coffee = parse_f32_field("ratio", &form.ratio_coffee, &mut errors);
    let ratio_water = parse_f32_field("ratio", &form.ratio_water, &mut errors);
    let default_dose_g = parse_f32_field("default_dose_g", &form.default_dose_g, &mut errors);
    let day0_grind_size = parse_f32_field("age_fitting", &form.day0_grind_size, &mut errors);
    let day0_water_temp_c = parse_f32_field("age_fitting", &form.day0_water_temp_c, &mut errors);
    let day14_grind_size = parse_f32_field("age_fitting", &form.day14_grind_size, &mut errors);
    let day14_water_temp_c = parse_f32_field("age_fitting", &form.day14_water_temp_c, &mut errors);
    let priority = parse_u32_field("priority", &form.priority, &mut errors);
    if !errors.is_empty() {
        return Err(errors);
    }
    let (
        Some(pour_stages),
        Some(ratio_coffee),
        Some(ratio_water),
        Some(default_dose_g),
        Some(day0_grind_size),
        Some(day0_water_temp_c),
        Some(day14_grind_size),
        Some(day14_water_temp_c),
        Some(priority),
    ) = (
        pour_stages,
        ratio_coffee,
        ratio_water,
        default_dose_g,
        day0_grind_size,
        day0_water_temp_c,
        day14_grind_size,
        day14_water_temp_c,
        priority,
    )
    else {
        return Err(vec![FormValidationError {
            field: "numeric_fields".to_string(),
            message: "数值字段解析失败".to_string(),
        }]);
    };
    Ok(BrewingPlanFormInput {
        editing_id: form.editing_id.clone(),
        category_id: form.category_id.clone(),
        name: form.name.clone(),
        matching_attributes: form.matching_attributes.clone(),
        pour_stages,
        dripper: form.dripper.clone(),
        grinder_profile_id: blank_to_none(form.grinder_profile_id.clone()),
        ratio_coffee,
        ratio_water,
        default_dose_g,
        day0_grind_size,
        day0_water_temp_c,
        day14_grind_size,
        day14_water_temp_c,
        instructions: form.instructions.clone(),
        priority,
    })
}

fn parse_u8_field(field: &str, value: &str, errors: &mut Vec<FormValidationError>) -> Option<u8> {
    match value.trim().parse::<u8>() {
        Ok(parsed) => Some(parsed),
        Err(_) => {
            errors.push(FormValidationError {
                field: field.to_string(),
                message: "请输入有效数字".to_string(),
            });
            None
        }
    }
}

fn parse_u32_field(field: &str, value: &str, errors: &mut Vec<FormValidationError>) -> Option<u32> {
    match value.trim().parse::<u32>() {
        Ok(parsed) => Some(parsed),
        Err(_) => {
            errors.push(FormValidationError {
                field: field.to_string(),
                message: "请输入有效数字".to_string(),
            });
            None
        }
    }
}

fn parse_f32_field(field: &str, value: &str, errors: &mut Vec<FormValidationError>) -> Option<f32> {
    match value.trim().parse::<f32>() {
        Ok(parsed) => Some(parsed),
        Err(_) => {
            errors.push(FormValidationError {
                field: field.to_string(),
                message: "请输入有效数字".to_string(),
            });
            None
        }
    }
}

fn options_for_match_kind(state: &AppState, kind: BrewingMatchKind) -> Vec<CatalogOption> {
    match kind {
        BrewingMatchKind::BeanVariety => state
            .coffee_parameters
            .bean_varieties
            .iter()
            .filter(|item| !item.archived)
            .cloned()
            .collect(),
        BrewingMatchKind::ProcessingMethod => state
            .coffee_parameters
            .processing_methods
            .iter()
            .filter(|item| !item.archived)
            .cloned()
            .collect(),
        BrewingMatchKind::RoastLevel => state
            .coffee_parameters
            .roast_levels
            .iter()
            .filter(|item| !item.archived)
            .map(|item| CatalogOption {
                id: item.id.clone(),
                label: item.label.clone(),
                sort_order: item.sort_order,
                archived: item.archived,
            })
            .collect(),
    }
}

fn match_kind_label(kind: BrewingMatchKind) -> &'static str {
    match kind {
        BrewingMatchKind::BeanVariety => "豆种",
        BrewingMatchKind::ProcessingMethod => "处理法",
        BrewingMatchKind::RoastLevel => "烘焙度",
    }
}

fn match_kind_value(kind: BrewingMatchKind) -> &'static str {
    match kind {
        BrewingMatchKind::BeanVariety => "variety",
        BrewingMatchKind::ProcessingMethod => "processing",
        BrewingMatchKind::RoastLevel => "roast_level",
    }
}

fn parse_match_kind(value: &str) -> BrewingMatchKind {
    match value {
        "variety" => BrewingMatchKind::BeanVariety,
        "roast_level" => BrewingMatchKind::RoastLevel,
        _ => BrewingMatchKind::ProcessingMethod,
    }
}

fn option_label_by_kind(state: &AppState, kind: BrewingMatchKind, id: &str) -> String {
    match kind {
        BrewingMatchKind::BeanVariety => find_label(&state.coffee_parameters.bean_varieties, id),
        BrewingMatchKind::ProcessingMethod => {
            find_label(&state.coffee_parameters.processing_methods, id)
        }
        BrewingMatchKind::RoastLevel => state
            .coffee_parameters
            .roast_levels
            .iter()
            .find(|item| item.id == id)
            .map(|item| item.label.clone())
            .unwrap_or_else(|| id.to_string()),
    }
}

fn find_label(options: &[CatalogOption], id: &str) -> String {
    options
        .iter()
        .find(|item| item.id == id)
        .map(|item| item.label.clone())
        .unwrap_or_else(|| id.to_string())
}
