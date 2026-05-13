use std::cmp::Ordering;

use crate::domain::models::{AppState, RoastBatch, RoastLevelOption};

pub fn parse_agtron_score_input(input: &str) -> Result<Option<f32>, &'static str> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let score = trimmed
        .parse::<f32>()
        .map_err(|_| "请输入有效的 AG 色值")?;
    if !score.is_finite() || !(1.0..=150.0).contains(&score) {
        return Err("请输入 1-150 之间的 AG 色值");
    }
    Ok(Some(score))
}

pub fn parse_agtron_range_bounds(input: &str) -> Option<(Option<f32>, Option<f32>)> {
    let normalized = input
        .trim()
        .replace([' ', '–', '—', '－', '～', '~'], "");
    if normalized.is_empty() {
        return None;
    }
    if let Some(min) = normalized.strip_suffix('+') {
        return min.parse::<f32>().ok().map(|value| (Some(value), None));
    }
    if let Some((min, max)) = normalized.split_once('-') {
        let min = min.parse::<f32>().ok()?;
        let max = max.parse::<f32>().ok()?;
        if min > max {
            return None;
        }
        return Some((Some(min), Some(max)));
    }
    normalized
        .parse::<f32>()
        .ok()
        .map(|value| (Some(value), Some(value)))
}

pub fn roast_level_bounds(level: &RoastLevelOption) -> Option<(Option<f32>, Option<f32>)> {
    if level.agtron_min.is_some() || level.agtron_max.is_some() {
        Some((level.agtron_min, level.agtron_max))
    } else {
        parse_agtron_range_bounds(&level.agtron_range)
    }
}

pub fn match_roast_level<'a>(
    score: f32,
    levels: &'a [RoastLevelOption],
) -> Option<&'a RoastLevelOption> {
    let mut candidates: Vec<(&RoastLevelOption, f32, f32)> = levels
        .iter()
        .filter(|level| !level.archived)
        .filter_map(|level| {
            let (min, max) = roast_level_bounds(level)?;
            if !agtron_in_bounds(score, min, max) {
                return None;
            }
            Some((
                level,
                min.unwrap_or(f32::NEG_INFINITY),
                max.unwrap_or(f32::INFINITY),
            ))
        })
        .collect();
    candidates.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(Ordering::Equal)
            .then(left.2.partial_cmp(&right.2).unwrap_or(Ordering::Equal))
            .then(left.0.sort_order.cmp(&right.0.sort_order))
    });
    candidates.into_iter().next().map(|(level, _, _)| level)
}

pub fn resolve_batch_roast_level_id(batch: &RoastBatch, state: &AppState) -> Option<String> {
    if batch.agtron_score.is_some() {
        return batch.matched_roast_level_id.clone();
    }
    if batch.roast_level_id.is_some() {
        return batch.roast_level_id.clone();
    }
    state
        .roast_profiles
        .iter()
        .find(|profile| profile.id == batch.profile_id)
        .and_then(|profile| profile.roast_level_id.clone())
}

pub fn resolve_batch_roast_level_label(batch: &RoastBatch, state: &AppState) -> Option<String> {
    let level_id = resolve_batch_roast_level_id(batch, state)?;
    state
        .coffee_parameters
        .roast_levels
        .iter()
        .find(|level| level.id == level_id)
        .map(|level| level.label.clone())
}

fn agtron_in_bounds(score: f32, min: Option<f32>, max: Option<f32>) -> bool {
    if min.is_none() && max.is_none() {
        return false;
    }
    let min_ok = min.is_none_or(|value| score >= value);
    let max_ok = max.is_none_or(|value| score <= value);
    min_ok && max_ok
}

#[cfg(test)]
mod tests {
    use crate::domain::models::RoastLevelOption;

    use super::{match_roast_level, parse_agtron_range_bounds, parse_agtron_score_input};

    #[test]
    fn parse_agtron_score_accepts_blank_and_numeric_values() {
        assert_eq!(parse_agtron_score_input(""), Ok(None));
        assert_eq!(parse_agtron_score_input("92.5"), Ok(Some(92.5)));
    }

    #[test]
    fn parse_agtron_range_supports_closed_and_open_ranges() {
        assert_eq!(parse_agtron_range_bounds("90-95"), Some((Some(90.0), Some(95.0))));
        assert_eq!(parse_agtron_range_bounds("95+"), Some((Some(95.0), None)));
    }

    #[test]
    fn match_roast_level_prefers_higher_lower_bound_on_overlap() {
        let levels = vec![
            RoastLevelOption {
                id: "light-medium".to_string(),
                label: "浅中".to_string(),
                agtron_range: "80-90".to_string(),
                agtron_min: Some(80.0),
                agtron_max: Some(90.0),
                sort_order: 2,
                archived: false,
            },
            RoastLevelOption {
                id: "light".to_string(),
                label: "浅".to_string(),
                agtron_range: "90-95".to_string(),
                agtron_min: Some(90.0),
                agtron_max: Some(95.0),
                sort_order: 1,
                archived: false,
            },
        ];

        let matched = match_roast_level(90.0, &levels).expect("should match roast level");
        assert_eq!(matched.id, "light");
    }
}
