use std::collections::HashMap;

use chrono::Utc;
use dioxus::prelude::*;

use crate::domain::brewing_match::{
    build_all_brewing_recommendations, build_brewing_recommendations, calculate_age_days,
    parse_roasted_at_utc,
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
                let profile = state.roast_profiles.iter().find(|p| p.id == batch.profile_id);
                let is_pour_over = profile.map(|p| p.product_line == ProductLine::PourOver).unwrap_or(false);
                let is_espresso = profile.map(|p| p.product_line == ProductLine::Espresso).unwrap_or(false);

                let age_days = parse_roasted_at_utc(&batch.roasted_at)
                    .map(|roasted_at| calculate_age_days(roasted_at, now))
                    .unwrap_or(0.0);

                let base_dose = custom_doses.read().get(&batch.id).copied();
                let matched_recommendations = if is_pour_over {
                    build_brewing_recommendations(batch, &state, now, base_dose)
                } else {
                    Vec::new()
                };

                let has_recommendations = !matched_recommendations.is_empty();
                let show_all = *show_all_plans.read().get(&batch.id).unwrap_or(&false) || has_recommendations;
                let batch_id_for_toggle = batch.id.clone();

                let recommendations = if !has_recommendations && show_all {
                    build_all_brewing_recommendations(batch, &state, now, base_dose)
                } else {
                    matched_recommendations
                };

                rsx! {
                    section { class: "panel",
                        h3 { class: "panel-title", "{batch.batch_no}" }
                        p { class: "list-line", "品类：{profile.map(|p| p.display_name.as_str()).unwrap_or(\"未知\")}" }
                        p { class: "list-line", "养豆天数: {format_age_days(age_days)}" }

                        if is_espresso {
                            if let Some(note) = profile.and_then(|p| p.espresso_note.as_ref()) {
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

                                for rec in recommendations {
                                    div { class: "list-item",
                                        p { class: "list-line", "方案: {rec.plan_name}" }
                                        p { class: "list-line", "滤杯: {rec.dripper} / 磨豆机: {rec.grinder}" }
                                        p { class: "list-line", "研磨度: {rec.grind_size:.1} / 水温: {rec.water_temp_c:.1}C" }
                                        p { class: "list-line", "注水段数: {rec.pour_stages}" }
                                        p { class: "list-line",
                                            "粉量: {rec.dose_g:.1}g / 总水量: {rec.total_water_g:.1}g"
                                        }
                                    }
                                }

                                if has_recommendations {
                                    div { class: "grid",
                                        div {
                                            label { class: "field-label", "粉量微调 (g)" }
                                            div { class: "action-row",
                                                button {
                                                    class: "button button-secondary",
                                                    onclick: {
                                                        let batch_id = batch.id.clone();
                                                        move |_| {
                                                            let current = custom_doses.read().get(&batch_id).copied().unwrap_or(16.0);
                                                            let next = ((current - 0.1) * 10.0).round() / 10.0;
                                                            custom_doses.write().insert(batch_id.clone(), next.max(10.0));
                                                        }
                                                    },
                                                    "-0.1"
                                                }
                                                span { class: "list-line", "{base_dose.unwrap_or(16.0):.1}g" }
                                                button {
                                                    class: "button button-secondary",
                                                    onclick: {
                                                        let batch_id = batch.id.clone();
                                                        move |_| {
                                                            let current = custom_doses.read().get(&batch_id).copied().unwrap_or(16.0);
                                                            let next = ((current + 0.1) * 10.0).round() / 10.0;
                                                            custom_doses.write().insert(batch_id.clone(), next.min(25.0));
                                                        }
                                                    },
                                                    "+0.1"
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

fn format_age_days(age_days: f32) -> String {
    if age_days < 1.0 {
        format!("{:.1} 天", age_days)
    } else {
        format!("{:.0} 天", age_days)
    }
}
