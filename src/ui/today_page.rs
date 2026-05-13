use std::collections::HashMap;

use chrono::Utc;
use dioxus::prelude::*;

use crate::domain::brewing_match::{
    build_all_brewing_recommendations, build_brewing_recommendations, calculate_age_days,
    parse_roasted_at_utc, resolve_batch_display_name, resolve_batch_espresso_note,
    resolve_batch_product_line,
};
use crate::domain::inventory::visible_recommendation_batches;
use crate::domain::models::{AppState, ProductLine};

#[component]
pub fn TodayPage(app_state: Signal<AppState>) -> Element {
    let state = app_state();
    let mut custom_doses = use_signal(HashMap::<String, f32>::new);
    let mut show_all_plans = use_signal(HashMap::<String, bool>::new);

    let batches = visible_recommendation_batches(&state);
    let now = Utc::now();

    rsx! {
        if batches.is_empty() {
            section { class: "panel",
                p { class: "list-line", "今日暂无生效中的批次。请先入库。" }
            }
        }
        for batch in batches {
            {
                let product_line = resolve_batch_product_line(batch, &state);
                let is_pour_over = product_line == Some(ProductLine::PourOver);
                let is_espresso = product_line == Some(ProductLine::Espresso);
                let batch_display_name = resolve_batch_display_name(batch, &state);

                let age_days = parse_roasted_at_utc(&batch.roasted_at)
                    .map(|roasted_at| calculate_age_days(roasted_at, now))
                    .unwrap_or(0.0);

                let preferred_dose = custom_doses.read().get(&batch.id).copied();
                let matched_recommendations = if is_pour_over {
                    build_brewing_recommendations(batch, &state, now, preferred_dose)
                } else {
                    Vec::new()
                };

                let has_recommendations = !matched_recommendations.is_empty();
                let show_all = *show_all_plans.read().get(&batch.id).unwrap_or(&false) || has_recommendations;
                let batch_id_for_toggle = batch.id.clone();

                let recommendations = if !has_recommendations && show_all {
                    build_all_brewing_recommendations(batch, &state, now, preferred_dose)
                } else {
                    matched_recommendations
                };
                let has_any_recommendations = !recommendations.is_empty();
                let current_dose = recommendations
                    .first()
                    .map(|rec| rec.dose_g)
                    .or(preferred_dose)
                    .unwrap_or(16.0);
                let estimated_remaining = remaining_after_single_brew(batch.capacity_g, current_dose);
                let estimated_cups = estimated_servings(batch.capacity_g, current_dose);

                rsx! {
                    section { class: "panel",
                        h3 { class: "panel-title", "{batch.batch_no}" }
                        p { class: "list-line", "批次信息：{batch_display_name}" }
                        p { class: "list-line", "养豆天数: {format_age_days(age_days)}" }

                        if is_espresso {
                            if let Some(note) = resolve_batch_espresso_note(batch, &state) {
                                p { class: "list-line", "萃取备注: {note}" }
                            } else {
                                p { class: "list-line", "意式批次, 无额外冲煮备注。" }
                            }
                        }

                        if is_pour_over {
                            if !show_all {
                                p { class: "list-line", "当前无匹配冲煮方案。" }
                                button {
                                    class: "button button-secondary",
                                    onclick: move |_| { show_all_plans.write().insert(batch_id_for_toggle.clone(), true); },
                                    "查看全部方案"
                                }
                            } else {
                                if !has_recommendations {
                                    p { class: "list-line", "当前无匹配方案, 以下是全部方案:" }
                                }

                                for rec in recommendations.iter() {
                                    div { class: "list-item",
                                        p { class: "list-line", "方案: {rec.plan_name}" }
                                        p { class: "list-line", "滤杯: {rec.dripper} / 磨豆机: {rec.grinder}" }
                                        p { class: "list-line", "研磨度: {rec.grind_size:.1} / 水温: {rec.water_temp_c:.1}C" }
                                        p { class: "list-line", "注水段数: {rec.pour_stages}" }
                                        p { class: "list-line", "粉量: {rec.dose_g:.1}g / 总水量: {rec.total_water_g:.1}g" }
                                    }
                                }

                                if has_any_recommendations {
                                    div { class: "grid",
                                        div {
                                            label { class: "field-label", "粉量微调 (g)" }
                                            div { class: "stepper",
                                                button {
                                                    class: "button button-secondary stepper-button",
                                                    onclick: {
                                                        let batch_id = batch.id.clone();
                                                        move |_| {
                                                            let current = custom_doses.read().get(&batch_id).copied().unwrap_or(current_dose);
                                                            let next = ((current - 0.1) * 10.0).round() / 10.0;
                                                            custom_doses.write().insert(batch_id.clone(), next.max(10.0));
                                                        }
                                                    },
                                                    "−"
                                                }
                                                span { class: "stepper-value", "{current_dose:.1}g" }
                                                button {
                                                    class: "button button-secondary stepper-button",
                                                    onclick: {
                                                        let batch_id = batch.id.clone();
                                                        move |_| {
                                                            let current = custom_doses.read().get(&batch_id).copied().unwrap_or(current_dose);
                                                            let next = ((current + 0.1) * 10.0).round() / 10.0;
                                                            custom_doses.write().insert(batch_id.clone(), next.min(25.0));
                                                        }
                                                    },
                                                    "+"
                                                }
                                            }
                                            p { class: "section-helper", "单杯耗豆 {current_dose:.1}g；按当前粉量冲一杯后约剩 {estimated_remaining:.1}g；这一批约还能冲 {estimated_cups} 杯。" }
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

fn remaining_after_single_brew(capacity_g: f32, dose_g: f32) -> f32 {
    (capacity_g - dose_g).max(0.0)
}

fn estimated_servings(capacity_g: f32, dose_g: f32) -> u32 {
    if dose_g <= 0.0 {
        return 0;
    }
    (capacity_g / dose_g).floor().max(0.0) as u32
}

fn format_age_days(age_days: f32) -> String {
    if age_days < 1.0 {
        format!("{:.1} 天", age_days)
    } else {
        format!("{:.0} 天", age_days)
    }
}
