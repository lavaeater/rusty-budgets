use crate::cqrs::runtime::AsyncBudgetCommandsTrait;
use crate::models::{Currency, Money};
use calamine::{DataType, Reader, Xlsx, XlsxError, open_workbook, open_workbook_from_rs};
use chrono::{DateTime, NaiveDate, ParseError, Utc};
use dioxus::prelude::{debug, error, info};
use std::io::{Cursor, Error};
use std::path::Path;
use uuid::Uuid;

fn extract_transfer_account_number(description: &str) -> Option<String> {
    let desc = description.trim();
    if let Some(rest) = desc.strip_prefix("Överföring ") {
        let digits: String = rest.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.starts_with("915") {
            return Some(digits);
        }
    }
    None
}

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("Account number missing")]
    AccountNumberMissing,
    #[error("IO error: {0}")]
    IoError(std::io::Error),
    #[error("Calamine error: {0}")]
    CalamineError(calamine::Error),
    #[error("Xlsx error: {0}")]
    XlsxError(XlsxError),
    #[error("Parse error: {0}")]
    ParseError(ParseError),
}

impl From<std::io::Error> for ImportError {
    fn from(value: Error) -> Self {
        ImportError::IoError(value)
    }
}

impl From<calamine::Error> for ImportError {
    fn from(value: calamine::Error) -> Self {
        ImportError::CalamineError(value)
    }
}

impl From<XlsxError> for ImportError {
    fn from(value: XlsxError) -> Self {
        ImportError::XlsxError(value)
    }
}

impl From<ParseError> for ImportError {
    fn from(value: ParseError) -> Self {
        ImportError::ParseError(value)
    }
}

pub async fn import_from_path(
    path: &str,
    user_id: Uuid,
    budget_id: Uuid,
    runtime: &impl AsyncBudgetCommandsTrait,
) -> Result<(u64, u64, u64), ImportError> {
    let p = Path::new(path);
    debug!("Importing from path: {}", path);
    if p.exists() {
        debug!("Path does exist!");
        if p.is_dir() {
            debug!("Path is a dir!");
            let entries = p.read_dir()?;
            let mut total = (0u64, 0u64, 0u64);
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(s) = path.to_str() {
                    let r = import_from_skandia_excel(runtime, user_id, budget_id, s).await?;
                    total.0 += r.0;
                    total.1 += r.1;
                    total.2 += r.2;
                }
            }
            Ok(total)
        } else {
            import_from_skandia_excel(runtime, user_id, budget_id, path).await
        }
    } else {
        Err(ImportError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Path does not exist",
        )))
    }
}

pub async fn import_from_skandia_excel(
    runtime: &impl AsyncBudgetCommandsTrait,
    user_id: Uuid,
    budget_id: Uuid,
    path: &str,
) -> Result<(u64, u64, u64), ImportError> {
    let mut imported = 0u64;
    let mut not_imported = 0u64;
    let mut total_rows = 0u64;
    let mut excel: Xlsx<_> = open_workbook(path)?;
    if let Ok(r) = excel.worksheet_range("Kontoutdrag") {
        let mut account_number: Option<String> = None;

        for (row_num, row) in r.rows().enumerate() {
            debug!("Row data: {:#?}", row);
            if row_num == 0 {
                account_number = Some(row[1].to_string());
                let acct_no = row[1].to_string();
                let _ = runtime.ensure_account(user_id, budget_id, &acct_no, "Skandiabanken").await;
            } else if row_num > 3 && row.len() > 3 {
                let amount =
                    Money::new_cents((row[2].as_f64().unwrap() * 100.0) as i64, Currency::SEK);
                let balance =
                    Money::new_cents((row[3].as_f64().unwrap() * 100.0) as i64, Currency::SEK);
                let date_str = row[0].to_string();
                let naive_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?;
                let date: DateTime<Utc> = naive_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
                let description = row[1].to_string();

                if let Some(counterpart) = extract_transfer_account_number(&description) {
                    let _ = runtime
                        .ensure_account(user_id, budget_id, &counterpart, "Skandiabanken")
                        .await;
                }

                let acct_no = account_number.clone().ok_or(ImportError::AccountNumberMissing)?;
                match runtime
                    .add_transaction(user_id, budget_id, &acct_no, amount, balance, &description, date)
                    .await
                {
                    Ok(_) => { imported += 1; total_rows += 1; }
                    Err(_) => { not_imported += 1; total_rows += 1; }
                }
            }
        }
        info!(
            "Imported {} transactions, skipped {} transactions, total {} transactions",
            imported, not_imported, total_rows
        );
    }

    Ok((imported, not_imported, total_rows))
}

pub async fn import_from_skandia_excel_bytes(
    runtime: &impl AsyncBudgetCommandsTrait,
    user_id: Uuid,
    budget_id: Uuid,
    bytes: Vec<u8>,
) -> Result<(u64, u64, u64), ImportError> {
    let mut imported = 0u64;
    let mut not_imported = 0u64;
    let mut total_rows = 0u64;
    info!("Opening new cursor!");
    let cursor = Cursor::new(bytes);
    info!("Opening workbook!");
    let mut excel: Xlsx<_> = open_workbook_from_rs(cursor)?;
    if let Ok(r) = excel.worksheet_range("Kontoutdrag") {
        info!("Found worksheet!");
        let mut account_number: Option<String> = None;

        for (row_num, row) in r.rows().enumerate() {
            debug!("Row data: {:#?}", row);
            if row_num == 0 {
                account_number = Some(row[1].to_string());
                let acct_no = row[1].to_string();
                let _ = runtime.ensure_account(user_id, budget_id, &acct_no, "Skandiabanken").await;
            } else if row_num > 3 && row.len() > 3 {
                let amount =
                    Money::new_cents((row[2].as_f64().unwrap() * 100.0) as i64, Currency::SEK);
                let balance =
                    Money::new_cents((row[3].as_f64().unwrap() * 100.0) as i64, Currency::SEK);
                let date_str = row[0].to_string();
                let naive_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?;
                let date: DateTime<Utc> = naive_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
                let description = row[1].to_string();

                if let Some(counterpart) = extract_transfer_account_number(&description) {
                    let _ = runtime
                        .ensure_account(user_id, budget_id, &counterpart, "Skandiabanken")
                        .await;
                }

                let acct_no = account_number.clone().ok_or(ImportError::AccountNumberMissing)?;
                match runtime
                    .add_transaction(user_id, budget_id, &acct_no, amount, balance, &description, date)
                    .await
                {
                    Ok(_) => { imported += 1; total_rows += 1; }
                    Err(_) => { not_imported += 1; total_rows += 1; }
                }
            }
        }
        info!(
            "Imported {} transactions from bytes, skipped {} transactions, total {} transactions",
            imported, not_imported, total_rows
        );
    }

    Ok((imported, not_imported, total_rows))
}
