use chrono::NaiveDate;

use crate::domain::models::RoastBatch;

pub fn generate_batch_no(
    date: NaiveDate,
    batch_code: &str,
    existing_batches: &[RoastBatch],
) -> String {
    let date_prefix = date.format("%Y%m%d").to_string();
    let batch_code = normalize_batch_code(batch_code);
    let max_sequence = existing_batches
        .iter()
        .filter_map(|batch| parse_sequence_if_same_date(&batch.batch_no, &date_prefix))
        .max()
        .unwrap_or(0);
    let next_sequence = max_sequence + 1;
    format!("{date_prefix}-{batch_code}-{next_sequence:03}")
}

pub fn normalize_batch_code(batch_code: &str) -> String {
    batch_code
        .trim()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || is_cjk(*ch))
        .map(|ch| ch.to_ascii_uppercase())
        .collect()
}

fn parse_sequence_if_same_date(batch_no: &str, date_prefix: &str) -> Option<u32> {
    let mut parts = batch_no.split('-');
    let date_part = parts.next()?;
    let _batch_code_part = parts.next()?;
    let sequence_part = parts.next()?;
    if parts.next().is_some() || date_part != date_prefix {
        return None;
    }
    sequence_part.parse::<u32>().ok()
}

fn is_cjk(ch: char) -> bool {
    matches!(
        ch,
        '\u{3400}'..='\u{4DBF}'
            | '\u{4E00}'..='\u{9FFF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{20000}'..='\u{2A6DF}'
            | '\u{2A700}'..='\u{2B73F}'
            | '\u{2B740}'..='\u{2B81F}'
            | '\u{2B820}'..='\u{2CEAF}'
    )
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::generate_batch_no;
    use crate::domain::models::{BatchStatus, RoastBatch};

    #[test]
    fn generate_batch_no_increments_sequence_for_same_date() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 2).expect("valid date");
        let existing = vec![batch("20260502-YJPO-001"), batch("20260502-YJPO-002")];

        let next = generate_batch_no(date, "YJPO", &existing);

        assert_eq!(next, "20260502-YJPO-003");
    }

    #[test]
    fn generate_batch_no_resets_for_new_date() {
        let existing = vec![batch("20260502-YJPO-001"), batch("20260502-YJPO-002")];
        let next_date = NaiveDate::from_ymd_opt(2026, 5, 3).expect("valid date");

        let next = generate_batch_no(next_date, "YJPO", &existing);

        assert_eq!(next, "20260503-YJPO-001");
    }

    #[test]
    fn generate_batch_no_uses_global_daily_sequence_across_batch_codes() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 2).expect("valid date");
        let mut existing = vec![batch("20260502-YJPO-001")];
        let second = generate_batch_no(date, "YJPO", &existing);
        assert_eq!(second, "20260502-YJPO-002");

        existing.push(batch(&second));
        let third = generate_batch_no(date, "ESP", &existing);
        assert_eq!(third, "20260502-ESP-003");
    }

    #[test]
    fn generate_batch_no_normalizes_batch_code_for_stable_parsing() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 2).expect("valid date");

        let next = generate_batch_no(date, " 耶加-浅 po ", &[]);

        assert_eq!(next, "20260502-耶加浅PO-001");
    }

    fn batch(batch_no: &str) -> RoastBatch {
        RoastBatch {
            id: format!("batch-{batch_no}"),
            profile_id: "profile-1".to_string(),
            bean_id: "bean-1".to_string(),
            product_line: Some(crate::domain::models::ProductLine::PourOver),
            roast_level_id: Some("roast-level-light".to_string()),
            batch_code: "TEST".to_string(),
            roasted_at: "2026-05-02T08:00:00Z".to_string(),
            batch_no: batch_no.to_string(),
            status: BatchStatus::Active,
            agtron_score: None,
            matched_roast_level_id: None,
            notes: None,
            capacity_g: 100.0,
        }
    }
}
