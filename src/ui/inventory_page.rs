use dioxus::prelude::*;
use gloo_timers::future::sleep;
use std::time::Duration;

use crate::domain::inventory::{BatchFormError, create_batches};
use crate::domain::models::{AppState, BatchStatus, ProductLine, RoastProfile};
use crate::ui::catalog_state::{
    ArchiveTarget, PendingArchive, begin_pending_archive, cancel_pending_archive,
    commit_pending_archive, pending_archive_label,
};
use crate::ui::save_app_state;

#[derive(Clone, Default)]
pub struct InventoryFormState {
    pub profile_id: String,
    pub roasted_at: String,
    pub count: String,
    pub notes: String,
}

fn start_pending_archive_countdown(
    mut app_state: Signal<AppState>,
    mut pending_archive: Signal<Option<PendingArchive>>,
    store_id: Signal<String>,
    mut save_status: Signal<Option<String>>,
) {
    spawn(async move {
        for _ in 0..5 {
            sleep(Duration::from_secs(1)).await;
            if let Some(ref mut p) = pending_archive.write().as_mut() {
                if p.remaining_seconds > 0 {
                    p.remaining_seconds -= 1;
                }
            } else {
                break;
            }
        }
        if let Some(p) = pending_archive() {
            if p.remaining_seconds == 0 {
                let committed = {
                    let mut state = app_state.write();
                    commit_pending_archive(&mut state, p).is_ok()
                };
                if committed {
                    pending_archive.set(None);
                    let saved_state = app_state.read().clone();
                    let sid = store_id();
                    spawn(async move {
                        save_status.set(Some("保存中...".to_string()));
                        match save_app_state(&saved_state, &sid).await {
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
            }
        }
    });
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
                                    let mut save_status_signal = save_status;
                                    spawn(async move {
                                        save_status_signal.set(Some("保存中...".to_string()));
                                        match save_app_state(&state, &sid).await {
                                            Ok(saved) => {
                                                app_state.set(saved);
                                                save_status_signal.set(Some("保存成功".to_string()));
                                            }
                                            Err(e) => save_status_signal.set(Some(e)),
                                        }
                                        sleep(Duration::from_secs(3)).await;
                                        save_status_signal.set(None);
                                    });
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

    let profiles: Vec<&RoastProfile> = state
        .roast_profiles
        .iter()
        .filter(|profile| !profile.archived)
        .collect();

    let mut sorted_batches: Vec<&crate::domain::models::RoastBatch> =
        state.batches.iter().collect();
    sorted_batches.sort_by(|left, right| {
        right
            .roasted_at
            .cmp(&left.roasted_at)
            .then(right.batch_no.cmp(&left.batch_no))
    });

    rsx! {
        {archive_section}
        section { class: "panel",
            h2 { class: "panel-title", "入库表单" }
            div { class: "grid",
                div {
                    label { class: "field-label", "烘焙品类" }
                    select {
                        class: "select-input",
                        value: "{form.read().profile_id}",
                        onchange: move |event| form.write().profile_id = event.value(),
                        option { value: "", "请选择品类" }
                        for profile in profiles.iter() {
                            option { value: "{profile.id}", "{profile.display_name}" }
                        }
                    }
                    InventoryFieldError { errors, field: "profile_id" }
                }
                div {
                    label { class: "field-label", "烘焙完成时间" }
                    input {
                        class: "text-input",
                        r#type: "datetime-local",
                        value: "{form.read().roasted_at}",
                        oninput: move |event| form.write().roasted_at = event.value(),
                    }
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
                            let count = form.read().count.trim().parse::<u32>().unwrap_or(0);
                            let notes_owned = form.read().notes.trim().to_string();
                            let notes = if notes_owned.is_empty() { None } else { Some(notes_owned.as_str()) };
                            let result = create_batches(
                                &mut app_state.write(),
                                &form.read().profile_id,
                                &form.read().roasted_at,
                                count,
                                notes,
                            );
                            match result {
                                Ok(_) => {
                                    errors.set(Vec::new());
                                    let state = app_state.read().clone();
                                    let sid = store_id();
                                    let mut save_status_signal = save_status;
                                    spawn(async move {
                                        save_status_signal.set(Some("保存中...".to_string()));
                                        match save_app_state(&state, &sid).await {
                                            Ok(saved) => {
                                                app_state.set(saved);
                                                save_status_signal.set(Some("保存成功".to_string()));
                                            }
                                            Err(e) => save_status_signal.set(Some(e)),
                                        }
                                        sleep(Duration::from_secs(3)).await;
                                        save_status_signal.set(None);
                                    });
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
                for batch in sorted_batches {
                    {
                        let profile = state.roast_profiles.iter().find(|p| p.id == batch.profile_id);
                        let product_line_label = profile.map(|p| match p.product_line {
                            ProductLine::PourOver => "手冲",
                            ProductLine::Espresso => "意式",
                        }).unwrap_or("未知");
                        let profile_name = profile.map(|p| p.display_name.as_str()).unwrap_or("未知品类");
                        rsx! {
                            p { class: "list-line", "批次号: {batch.batch_no}" }
                            p { class: "list-line", "品类: {profile_name} ({product_line_label})" }
                            p { class: "list-line", "烘焙时间: {roasted_date(batch.roasted_at.as_str())}" }
                            p { class: "list-line", "容量: {batch.capacity_g}g" }
                            p { class: "list-line",
                                match batch.status {
                                    BatchStatus::Active => "状态: 生效中",
                                    BatchStatus::UsedUp => "状态: 已用完",
                                    BatchStatus::Archived => "状态: 已归档",
                                }
                            }
                            if let Some(notes) = &batch.notes {
                                p { class: "list-line", "备注: {notes}" }
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
                                                    start_pending_archive_countdown(
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
                                                    start_pending_archive_countdown(
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
