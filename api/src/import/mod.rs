use crate::cqrs::framework::{AsyncRuntime, CommandError, Runtime};
use crate::models::{Budget, BudgetEvent, Currency, Money};
use calamine::{Data, DataType, Range, Reader, Xlsx, XlsxError, open_workbook, open_workbook_from_rs};
use chrono::{DateTime, NaiveDate, ParseError, Utc};
use dioxus::prelude::{debug, info};
use std::collections::HashSet;
use std::io::{Cursor, Error};
use std::path::Path;
use uuid::Uuid;

fn extract_transfer_account_number(description: &str) -> Option<String> {
    let desc = description.trim();
    if let Some(rest) = desc.strip_prefix("Överföring ") {
        let digits: String = rest.chars().filter(char::is_ascii_digit).collect();
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

/// One data row from the "Kontoutdrag" sheet, already parsed into domain
/// types. Building this without touching the runtime lets us decide *all*
/// the commands for a whole file before ever loading the aggregate.
struct ParsedTransactionRow {
    account_number: String,
    counterpart_account_number: Option<String>,
    amount: Money,
    balance: Money,
    description: String,
    date: DateTime<Utc>,
}

struct ParsedSkandiaSheet {
    primary_account_number: Option<String>,
    rows: Vec<ParsedTransactionRow>,
}

/// # Panics
/// Panics if an amount/balance cell cannot be parsed as a number.
#[allow(clippy::cast_possible_truncation)]
fn parse_skandia_sheet(range: &Range<Data>) -> Result<ParsedSkandiaSheet, ImportError> {
    let mut account_number: Option<String> = None;
    let mut rows = Vec::new();

    for (row_num, row) in range.rows().enumerate() {
        debug!("Row data: {:#?}", row);
        if row_num == 0 {
            account_number = Some(row[1].to_string());
        } else if row_num > 3 && row.len() > 3 {
            let amount = Money::new_cents((row[2].as_f64().unwrap() * 100.0) as i64, Currency::SEK);
            let balance = Money::new_cents((row[3].as_f64().unwrap() * 100.0) as i64, Currency::SEK);
            let date_str = row[0].to_string();
            let naive_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?;
            let date: DateTime<Utc> = naive_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
            let description = row[1].to_string();
            let counterpart_account_number = extract_transfer_account_number(&description);
            let acct_no = account_number
                .clone()
                .ok_or(ImportError::AccountNumberMissing)?;

            rows.push(ParsedTransactionRow {
                account_number: acct_no,
                counterpart_account_number,
                amount,
                balance,
                description,
                date,
            });
        }
    }

    Ok(ParsedSkandiaSheet {
        primary_account_number: account_number,
        rows,
    })
}

/// Whether a queued command is a bank-account creation (whose outcome we
/// don't report to the caller) or a transaction import (whose outcome feeds
/// the `(imported, not_imported)` counters).
enum RowCommandKind {
    Account,
    Transaction,
}

type BudgetCommand = Box<dyn FnOnce(&Budget) -> Result<BudgetEvent, CommandError> + Send>;

/// Turns a parsed sheet into the full list of commands needed to import it,
/// in row order. Account-creation commands are deduplicated locally so a
/// counterpart account seen on every other row only produces one command,
/// not one per occurrence. Whether the account already exists in the budget
/// is decided later, in-memory, when the command actually runs.
fn build_import_commands(sheet: &ParsedSkandiaSheet) -> (Vec<BudgetCommand>, Vec<RowCommandKind>) {
    let mut commands: Vec<BudgetCommand> = Vec::new();
    let mut kinds = Vec::new();
    let mut queued_accounts: HashSet<String> = HashSet::new();

    let queue_account = |commands: &mut Vec<BudgetCommand>,
                              kinds: &mut Vec<RowCommandKind>,
                              queued_accounts: &mut HashSet<String>,
                              account_number: String| {
        if queued_accounts.insert(account_number.clone()) {
            commands.push(Box::new(move |budget: &Budget| {
                budget
                    .create_bank_account(account_number, "Skandiabanken".to_string())
                    .map(Into::into)
            }));
            kinds.push(RowCommandKind::Account);
        }
    };

    if let Some(primary) = &sheet.primary_account_number {
        queue_account(
            &mut commands,
            &mut kinds,
            &mut queued_accounts,
            primary.clone(),
        );
    }

    for row in &sheet.rows {
        if let Some(counterpart) = &row.counterpart_account_number {
            queue_account(
                &mut commands,
                &mut kinds,
                &mut queued_accounts,
                counterpart.clone(),
            );
        }

        let ParsedTransactionRow {
            account_number,
            amount,
            balance,
            description,
            date,
            ..
        } = row;
        let account_number = account_number.clone();
        let description = description.clone();
        let amount = *amount;
        let balance = *balance;
        let date = *date;
        commands.push(Box::new(move |budget: &Budget| {
            budget
                .add_transaction(account_number, amount, balance, description, date)
                .map(Into::into)
        }));
        kinds.push(RowCommandKind::Transaction);
    }

    (commands, kinds)
}

fn tally_transaction_results(
    kinds: &[RowCommandKind],
    results: &[Result<Uuid, CommandError>],
) -> (u64, u64, u64) {
    let mut imported = 0u64;
    let mut not_imported = 0u64;
    let mut total_rows = 0u64;
    for (kind, result) in kinds.iter().zip(results.iter()) {
        if matches!(kind, RowCommandKind::Transaction) {
            total_rows += 1;
            if result.is_ok() {
                imported += 1;
            } else {
                not_imported += 1;
            }
        }
    }
    (imported, not_imported, total_rows)
}

pub async fn import_from_path(
    path: &str,
    user_id: Uuid,
    budget_id: Uuid,
    runtime: &impl AsyncRuntime<Budget, BudgetEvent>,
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

/// Imports a whole sheet with a single `load` and a single `snapshot`,
/// instead of one of each per row. See `Runtime::bulk_execute` /
/// `AsyncRuntime::bulk_execute` for why that matters: going through the
/// per-command `execute` path for every row reloads and replays the entire
/// event stream on every single call, which turns a multi-year import into
/// an O(rows²) operation.
async fn run_bulk_import(
    runtime: &impl AsyncRuntime<Budget, BudgetEvent>,
    user_id: Uuid,
    budget_id: Uuid,
    sheet: &ParsedSkandiaSheet,
) -> (u64, u64, u64) {
    let (commands, kinds) = build_import_commands(sheet);
    let results = runtime
        .bulk_execute(user_id, budget_id, commands)
        .await
        .unwrap_or_default();
    tally_transaction_results(&kinds, &results)
}

fn run_bulk_import_sync(
    runtime: &impl Runtime<Budget, BudgetEvent>,
    user_id: Uuid,
    budget_id: Uuid,
    sheet: &ParsedSkandiaSheet,
) -> (u64, u64, u64) {
    let (commands, kinds) = build_import_commands(sheet);
    let results = runtime
        .bulk_execute(user_id, budget_id, commands)
        .unwrap_or_default();
    tally_transaction_results(&kinds, &results)
}

/// # Panics
/// Panics if an amount/balance cell cannot be parsed as a number.
pub async fn import_from_skandia_excel(
    runtime: &impl AsyncRuntime<Budget, BudgetEvent>,
    user_id: Uuid,
    budget_id: Uuid,
    path: &str,
) -> Result<(u64, u64, u64), ImportError> {
    let mut excel: Xlsx<_> = open_workbook(path)?;
    let Ok(range) = excel.worksheet_range("Kontoutdrag") else {
        return Ok((0, 0, 0));
    };
    let sheet = parse_skandia_sheet(&range)?;
    let (imported, not_imported, total_rows) =
        run_bulk_import(runtime, user_id, budget_id, &sheet).await;
    info!(
        "Imported {} transactions, skipped {} transactions, total {} transactions",
        imported, not_imported, total_rows
    );
    Ok((imported, not_imported, total_rows))
}

/// # Panics
/// Panics if an amount/balance cell cannot be parsed as a number.
pub fn import_from_skandia_excel_sync(
    runtime: &impl Runtime<Budget, BudgetEvent>,
    user_id: Uuid,
    budget_id: Uuid,
    path: &str,
) -> Result<(u64, u64, u64), ImportError> {
    let mut excel: Xlsx<_> = open_workbook(path)?;
    let Ok(range) = excel.worksheet_range("Kontoutdrag") else {
        return Ok((0, 0, 0));
    };
    let sheet = parse_skandia_sheet(&range)?;
    Ok(run_bulk_import_sync(runtime, user_id, budget_id, &sheet))
}

/// # Panics
/// Panics if an amount/balance cell cannot be parsed as a number.
pub async fn import_from_skandia_excel_bytes(
    runtime: &impl AsyncRuntime<Budget, BudgetEvent>,
    user_id: Uuid,
    budget_id: Uuid,
    bytes: Vec<u8>,
) -> Result<(u64, u64, u64), ImportError> {
    info!("Opening new cursor!");
    let cursor = Cursor::new(bytes);
    info!("Opening workbook!");
    let mut excel: Xlsx<_> = open_workbook_from_rs(cursor)?;
    let Ok(range) = excel.worksheet_range("Kontoutdrag") else {
        return Ok((0, 0, 0));
    };
    info!("Found worksheet!");
    let sheet = parse_skandia_sheet(&range)?;
    let (imported, not_imported, total_rows) =
        run_bulk_import(runtime, user_id, budget_id, &sheet).await;
    info!(
        "Imported {} transactions from bytes, skipped {} transactions, total {} transactions",
        imported, not_imported, total_rows
    );
    Ok((imported, not_imported, total_rows))
}
