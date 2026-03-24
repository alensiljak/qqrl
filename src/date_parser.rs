use chrono::{Duration, NaiveDate};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DateParseError {
    #[error("Invalid date format: {0}")]
    InvalidDateFormat(String),
}

/// Parse YYYY, YYYY-MM, or YYYY-MM-DD into a full date string.
pub fn parse_date(date_str: &str) -> Result<String, DateParseError> {
    let parts: Vec<&str> = date_str.split('-').collect();

    match parts.len() {
        1 => {
            let year: i32 = parts[0]
                .parse()
                .map_err(|_| DateParseError::InvalidDateFormat(date_str.to_string()))?;
            Ok(format!("{year:04}-01-01"))
        }
        2 => {
            let year: i32 = parts[0]
                .parse()
                .map_err(|_| DateParseError::InvalidDateFormat(date_str.to_string()))?;
            let month: u32 = parts[1]
                .parse()
                .map_err(|_| DateParseError::InvalidDateFormat(date_str.to_string()))?;
            Ok(format!("{year:04}-{month:02}-01"))
        }
        3 => Ok(date_str.to_string()),
        _ => Err(DateParseError::InvalidDateFormat(date_str.to_string())),
    }
}

/// Parse shorthand dates and ranges into `(begin, end)` where end is exclusive for single values.
pub fn parse_date_range(
    date_range_str: &str,
) -> Result<(Option<String>, Option<String>), DateParseError> {
    if date_range_str.contains("..") {
        let parts: Vec<&str> = date_range_str.split("..").collect();
        if parts.len() != 2 {
            return Err(DateParseError::InvalidDateFormat(
                date_range_str.to_string(),
            ));
        }

        let start_part = parts[0];
        let end_part = parts[1];

        let start_date = if start_part.is_empty() {
            None
        } else {
            Some(parse_date(start_part)?)
        };

        let end_date = if end_part.is_empty() {
            None
        } else {
            Some(parse_date(end_part)?)
        };

        return Ok((start_date, end_date));
    }

    let start = parse_date(date_range_str)?;
    let parts: Vec<&str> = date_range_str.split('-').collect();

    let end = match parts.len() {
        1 => {
            let year: i32 = parts[0]
                .parse()
                .map_err(|_| DateParseError::InvalidDateFormat(date_range_str.to_string()))?;
            format!("{:04}-01-01", year + 1)
        }
        2 => {
            let year: i32 = parts[0]
                .parse()
                .map_err(|_| DateParseError::InvalidDateFormat(date_range_str.to_string()))?;
            let month: u32 = parts[1]
                .parse()
                .map_err(|_| DateParseError::InvalidDateFormat(date_range_str.to_string()))?;

            let (next_year, next_month) = if month >= 12 {
                (year + 1, 1)
            } else {
                (year, month + 1)
            };

            format!("{next_year:04}-{next_month:02}-01")
        }
        3 => {
            let date = NaiveDate::parse_from_str(date_range_str, "%Y-%m-%d")
                .map_err(|_| DateParseError::InvalidDateFormat(date_range_str.to_string()))?;
            let next = date + Duration::days(1);
            next.format("%Y-%m-%d").to_string()
        }
        _ => {
            return Err(DateParseError::InvalidDateFormat(
                date_range_str.to_string(),
            ))
        }
    };

    Ok((Some(start), Some(end)))
}

#[cfg(test)]
mod tests {
    use super::{parse_date, parse_date_range};

    #[test]
    fn parse_date_basic_formats() {
        assert_eq!(parse_date("2025").unwrap(), "2025-01-01");
        assert_eq!(parse_date("2025-08").unwrap(), "2025-08-01");
        assert_eq!(parse_date("2025-08-15").unwrap(), "2025-08-15");
    }

    #[test]
    fn parse_date_range_shorthand() {
        assert_eq!(
            parse_date_range("2025").unwrap(),
            (
                Some("2025-01-01".to_string()),
                Some("2026-01-01".to_string())
            )
        );
        assert_eq!(
            parse_date_range("2025-08").unwrap(),
            (
                Some("2025-08-01".to_string()),
                Some("2025-09-01".to_string())
            )
        );
        assert_eq!(
            parse_date_range("2025-08-15").unwrap(),
            (
                Some("2025-08-15".to_string()),
                Some("2025-08-16".to_string())
            )
        );
    }

    #[test]
    fn parse_date_range_interval_syntax() {
        assert_eq!(
            parse_date_range("2025-08..").unwrap(),
            (Some("2025-08-01".to_string()), None)
        );
        assert_eq!(
            parse_date_range("..2025-09").unwrap(),
            (None, Some("2025-09-01".to_string()))
        );
        assert_eq!(
            parse_date_range("2025..2026").unwrap(),
            (
                Some("2025-01-01".to_string()),
                Some("2026-01-01".to_string())
            )
        );
    }
}
