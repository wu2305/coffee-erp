use dioxus::prelude::*;

use crate::domain::agtron::{
    match_roast_level, parse_agtron_score_input, resolve_batch_roast_level_label,
};
use crate::domain::brewing_match::resolve_batch_display_name;
use crate::domain::inventory::{BatchFormError, create_batches};
use crate::domain::models::{AppState, BatchStatus, ProductLine, RoastBatch};
use crate::ui::catalog_state::{
    ArchiveTarget, PendingArchive, begin_pending_archive, cancel_pending_archive,
    commit_pending_archive, pending_archive_label, suggest_inventory_batch_code,
};
use crate::ui::save_with_rollback;

#[derive(Clone)]
pub struct InventoryFormState {
    pub bean_id: String,
    pub product_line: ProductLine,
    pub batch_code: String,
    pub batch_code_customized: bool,
    pub roasted_at: String,
    pub count: String,
    pub agtron_score: String,
    pub notes: String,
}

impl Default for InventoryFormState {
    fn default() -> Self {
        Self {
            bean_id: String::new(),
            product_line: ProductLine::PourOver,
            batch_code: String::new(),
            batch_code_customized: false,
            roasted_at: today_local_date(),
            count: "1".to_string(),
            agtron_score: String::new(),
            notes: String::new(),
        }
    }
}

#[component]
pub fn InventoryPage(
    app_state: Signal<AppState>,
    pending_archive: Signal<Option<PendingArchive>>,
    store_id: Signal<String>,
    save_status: Signal<Option<String>>,
) -> Element {
    let mut form = use_signal(InventoryFormState::default);
    let mut errors = use_signal(Vec::<BatchFormError>::new);

    let state = app_state();
    let archive_pending = pending_archive();
    let archive_locked = pending_archive().is_some();

    let archive_section = match &archive_pending {
        Some(pending) => rsx! {
            section { class: "pending",
                p { class: "pending-text", "待操作：{pending_archive_label(&state, pending)}。{pending.remaining_seconds} 秒后可撤销或立即提交。" }
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
                                let before_state = app_state.read().clone();
                                let new_state = {
                                    let mut current = app_state.write();
                                    if commit_pending_archive(&mut current, pending).is_ok() {
                                        pending_archive.set(None);
                                        Some(current.clone())
                                    } else {
                                        None
                                    }
                                };
                                if new_state.is_some() {
                                    save_with_rollback(
                                        app_state,
                                        before_state,
                                        store_id,
                                        save_status,
                                    );
                                }
                            }
                        },
                        "确认提交"
                    }
                }
            }
        },
        None => rsx! {},
    };

    let beans: Vec<&crate::domain::models::CoffeeBean> =
        state.beans.iter().filter(|bean| !bean.archived).collect();

    let mut sorted_batches: Vec<&RoastBatch> = state.batches.iter().collect();
    sorted_batches.sort_by(|left, right| {
        right
            .roasted_at
            .cmp(&left.roasted_at)
            .then(right.batch_no.cmp(&left.batch_no))
    });
    let grouped_batches = group_inventory_batches(&sorted_batches);

    let agtron_preview = match parse_agtron_score_input(form.read().agtron_score.as_str()) {
        Ok(Some(score)) => match match_roast_level(score, &state.coffee_parameters.roast_levels) {
            Some(level) => Some(format!("将自动匹配为：{}（AG {:.1}）", level.label, score)),
            None => Some("当前未匹配到烘焙度，请检查参数目录中的 Agtron 范围".to_string()),
        },
        Ok(None) => Some("填写 AG 色值后，系统会自动匹配烘焙度。".to_string()),
        Err(message) => Some(message.to_string()),
    };

    rsx! {
        {archive_section}
        section { class: "panel",
            h2 { class: "panel-title", "入库表单" }
            div { class: "grid",
                div {
                    label { class: "field-label", "咖啡豆" }
                    select {
                        class: "select-input",
                        value: "{form.read().bean_id}",
                        onchange: move |event| {
                            let bean_id = event.value();
                            let should_refresh = !form.read().batch_code_customized;
                            form.write().bean_id = bean_id;
                            if should_refresh {
                                refresh_inventory_batch_code(app_state, form);
                            }
                        },
                        option { value: "", "请选择咖啡豆" }
                        for bean in beans.iter() {
                            option { value: "{bean.id}", "{bean.name}" }
                        }
                    }
                    InventoryFieldError { errors, field: "bean_id" }
                }
                div {
                    label { class: "field-label", "产品线" }
                    select {
                        class: "select-input",
                        value: if form.read().product_line == ProductLine::PourOver { "pour_over" } else { "espresso" },
                        onchange: move |event| {
                            let next_line = if event.value() == "espresso" {
                                ProductLine::Espresso
                            } else {
                                ProductLine::PourOver
                            };
                            form.write().product_line = next_line;
                        },
                        option { value: "pour_over", "手冲" }
                        option { value: "espresso", "意式" }
                    }
                }
                div {
                    label { class: "field-label", "批次编码" }
                    div { class: "field-with-action",
                        input {
                            class: "text-input",
                            value: "{form.read().batch_code}",
                            oninput: move |event| {
                                form.write().batch_code = event.value();
                                form.write().batch_code_customized = true;
                            },
                        }
                        button {
                            class: "button button-secondary field-action-button",
                            r#type: "button",
                            disabled: archive_locked,
                            title: "重新生成建议批次编码",
                            onclick: move |_| {
                                refresh_inventory_batch_code(app_state, form);
                                form.write().batch_code_customized = false;
                            },
                            "↻"
                        }
                    }
                    p { class: "section-helper", "右侧图标会按当前豆子、AG 匹配烘焙度与处理法重新生成建议编码。" }
                    InventoryFieldError { errors, field: "batch_code" }
                }
                div {
                    label { class: "field-label", "烘焙日期" }
                    div { class: "field-with-action",
                        input {
                            class: "text-input",
                            r#type: "date",
                            value: "{form.read().roasted_at}",
                            oninput: move |event| form.write().roasted_at = event.value(),
                        }
                        span { class: "field-icon", "▾" }
                    }
                    p { class: "section-helper", "默认带出今天，点一下日期栏即可在移动端选择日期。" }
                    InventoryFieldError { errors, field: "roasted_at" }
                }
                div {
                    label { class: "field-label", "批次数量 (每批 100g)" }
                    input {
                        class: "number-input",
                        r#type: "number",
                        min: "1",
                        value: "{form.read().count}",
                        oninput: move |event| form.write().count = event.value(),
                    }
                    InventoryFieldError { errors, field: "count" }
                }
                div {
                    label { class: "field-label", "AG 色值" }
                    input {
                        class: "number-input",
                        r#type: "number",
                        min: "1",
                        max: "150",
                        step: "0.1",
                        value: "{form.read().agtron_score}",
                        oninput: move |event| {
                            let should_refresh = !form.read().batch_code_customized;
                            form.write().agtron_score = event.value();
                            if should_refresh {
                                refresh_inventory_batch_code(app_state, form);
                            }
                        },
                    }
                    if let Some(preview) = agtron_preview.as_ref() {
                        p { class: "section-helper", "{preview}" }
                    }
                    InventoryFieldError { errors, field: "agtron_score" }
                }
                div {
                    label { class: "field-label", "备注" }
                    textarea {
                        class: "textarea-input",
                        value: "{form.read().notes}",
                        oninput: move |event| form.write().notes = event.value(),
                    }
                }
                div { class: "action-row",
                    button {
                        class: "button",
                        disabled: archive_locked,
                        onclick: move |_| {
                            let form_snapshot = form.read().clone();
                            let count = form_snapshot.count.trim().parse::<u32>().unwrap_or(0);
                            let agtron_score = match parse_agtron_score_input(form_snapshot.agtron_score.as_str()) {
                                Ok(value) => value,
                                Err(message) => {
                                    errors.set(vec![BatchFormError::new("agtron_score", message)]);
                                    return;
                                }
                            };
                            let notes_owned = form_snapshot.notes.trim().to_string();
                            let notes = if notes_owned.is_empty() { None } else { Some(notes_owned.as_str()) };
                            let before_state = app_state.read().clone();
                            let result = {
                                let mut state = app_state.write();
                                create_batches(
                                    &mut state,
                                    &form_snapshot.bean_id,
                                    form_snapshot.product_line,
                                    None,
                                    &form_snapshot.batch_code,
                                    &form_snapshot.roasted_at,
                                    count,
                                    agtron_score,
                                    notes,
                                )
                            };
                            match result {
                                Ok(_) => {
                                    errors.set(Vec::new());
                                    save_with_rollback(
                                        app_state,
                                        before_state,
                                        store_id,
                                        save_status,
                                    );
                                }
                                Err(validation_errors) => {
                                    errors.set(validation_errors);
                                }
                            }
                        },
                        "保存入库"
                    }
                    button {
                        class: "button button-secondary",
                        disabled: archive_locked,
                        onclick: move |_| {
                            form.set(InventoryFormState::default());
                            errors.set(Vec::new());
                        },
                        "清空"
                    }
                }
            }
        }
        section { class: "panel",
            h2 { class: "panel-title", "批次列表" }
            div { class: "list",
                if grouped_batches.is_empty() {
                    p { class: "list-line", "暂无批次。请先入库。" }
                }
                for group in grouped_batches {
                    {
                        let summary_batch = group.summary_batch;
                        let batch_display_name = resolve_batch_display_name(summary_batch, &state);
                        let batch_code = resolve_batch_code(summary_batch);
                        let total_capacity: f32 = group.batches.iter().map(|batch| batch.capacity_g).sum();
                        let batch_numbers = group
                            .batches
                            .iter()
                            .map(|batch| batch.batch_no.as_str())
                            .collect::<Vec<_>>()
                            .join("、");
                        rsx! {
                            div { class: "list-item",
                                p { class: "list-line", "批次信息: {batch_display_name}" }
                                if group.batches.len() > 1 {
                                    p { class: "list-line", "本次已合并展示 {group.batches.len()} 批 · 总容量 {total_capacity:.0}g" }
                                } else {
                                    p { class: "list-line", "容量: {total_capacity:.0}g" }
                                }
                                if !batch_code.is_empty() {
                                    p { class: "list-line", "批次编码: {batch_code}" }
                                }
                                p { class: "list-line", "烘焙日期: {roasted_date(summary_batch.roasted_at.as_str())}" }
                                if let Some(score) = summary_batch.agtron_score {
                                    p { class: "list-line", "AG 色值: {format_agtron_score(score)}" }
                                }
                                if let Some(roast_level_label) = resolve_batch_roast_level_label(summary_batch, &state) {
                                    p { class: "list-line", "烘焙度: {roast_level_label}" }
                                }
                                if let Some(notes) = &summary_batch.notes {
                                    p { class: "list-line", "备注: {notes}" }
                                }
                                if group.batches.len() > 1 {
                                    p { class: "list-line", "批次号: {batch_numbers}" }
                                }
                                div { class: "list",
                                    for batch in group.batches {
                                        div { class: "list-item",
                                            p { class: "list-line", "批次号: {batch.batch_no}" }
                                            p { class: "list-line",
                                                match batch.status {
                                                    BatchStatus::Active => "状态: 生效中",
                                                    BatchStatus::UsedUp => "状态: 已用完",
                                                    BatchStatus::Archived => "状态: 已归档",
                                                }
                                            }
                                            div { class: "action-row",
                                                if batch.status == BatchStatus::Active {
                                                    button {
                                                        class: "button button-secondary",
                                                        disabled: archive_locked,
                                                        onclick: {
                                                            let batch_id = batch.id.clone();
                                                            move |_| {
                                                                let current_pending = pending_archive();
                                                                if let Ok(pending) = begin_pending_archive(
                                                                    &current_pending,
                                                                    ArchiveTarget::BatchUsedUp { id: batch_id.clone() },
                                                                ) {
                                                                    pending_archive.set(Some(pending));
                                                                    crate::ui::start_pending_archive_countdown(
                                                                        app_state,
                                                                        pending_archive,
                                                                        store_id,
                                                                        save_status,
                                                                    );
                                                                }
                                                            }
                                                        },
                                                        "标记用完"
                                                    }
                                                }
                                                if batch.status == BatchStatus::Active || batch.status == BatchStatus::UsedUp {
                                                    button {
                                                        class: "button button-danger",
                                                        disabled: archive_locked,
                                                        onclick: {
                                                            let batch_id = batch.id.clone();
                                                            move |_| {
                                                                let current_pending = pending_archive();
                                                                if let Ok(pending) = begin_pending_archive(
                                                                    &current_pending,
                                                                    ArchiveTarget::BatchArchived { id: batch_id.clone() },
                                                                ) {
                                                                    pending_archive.set(Some(pending));
                                                                    crate::ui::start_pending_archive_countdown(
                                                                        app_state,
                                                                        pending_archive,
                                                                        store_id,
                                                                        save_status,
                                                                    );
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
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn InventoryFieldError(errors: Signal<Vec<BatchFormError>>, field: &'static str) -> Element {
    let error_message = errors
        .read()
        .iter()
        .find(|error| error.field == field)
        .map(|error| error.message.clone());
    match error_message {
        Some(message) => rsx! { p { class: "error-text", "{message}" } },
        None => rsx! {},
    }
}

fn roasted_date(roasted_at: &str) -> &str {
    roasted_at.split('T').next().unwrap_or("")
}

fn format_agtron_score(score: f32) -> String {
    format!("{score:.1}")
}

fn refresh_inventory_batch_code(app_state: Signal<AppState>, mut form: Signal<InventoryFormState>) {
    let state = app_state();
    let snapshot = form.read().clone();
    let selected_bean = state.beans.iter().find(|bean| bean.id == snapshot.bean_id);
    let Some(bean) = selected_bean else {
        return;
    };
    let bean_name = bean.name.as_str();
    let roast_level_label = parse_agtron_score_input(snapshot.agtron_score.as_str())
        .ok()
        .flatten()
        .and_then(|score| match_roast_level(score, &state.coffee_parameters.roast_levels))
        .map(|level| level.label.as_str());
    let processing_method_label = bean.processing_method_id.as_deref().and_then(|id| {
        state
            .coffee_parameters
            .processing_methods
            .iter()
            .find(|method| method.id == id && !method.archived)
            .map(|method| method.label.as_str())
    });
    let code = suggest_inventory_batch_code(bean_name, roast_level_label, processing_method_label);
    form.write().batch_code = code;
}

#[derive(Clone)]
struct BatchDisplayGroup<'a> {
    summary_batch: &'a RoastBatch,
    batches: Vec<&'a RoastBatch>,
}

fn group_inventory_batches<'a>(batches: &[&'a RoastBatch]) -> Vec<BatchDisplayGroup<'a>> {
    let mut groups: Vec<BatchDisplayGroup<'a>> = Vec::new();
    for batch in batches.iter().copied() {
        if let Some(last_group) = groups.last_mut()
            && should_merge_batches(last_group.summary_batch, batch)
        {
            last_group.batches.push(batch);
            continue;
        }
        groups.push(BatchDisplayGroup {
            summary_batch: batch,
            batches: vec![batch],
        });
    }
    groups
}

fn should_merge_batches(left: &RoastBatch, right: &RoastBatch) -> bool {
    left.status == right.status
        && left.bean_id == right.bean_id
        && left.product_line == right.product_line
        && left.roast_level_id == right.roast_level_id
        && resolve_batch_code(left) == resolve_batch_code(right)
        && roasted_date(left.roasted_at.as_str()) == roasted_date(right.roasted_at.as_str())
        && left.agtron_score == right.agtron_score
        && left.notes == right.notes
}

fn today_local_date() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let date = js_sys::Date::new_0();
        return format!(
            "{:04}-{:02}-{:02}",
            date.get_full_year(),
            date.get_month() + 1,
            date.get_date()
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        chrono::Local::now().format("%Y-%m-%d").to_string()
    }
}

fn resolve_batch_code(batch: &RoastBatch) -> String {
    if !batch.batch_code.trim().is_empty() {
        return batch.batch_code.clone();
    }
    batch.batch_no.split('-').nth(1).unwrap_or("").to_string()
}
