use crate::cqrs::framework::Runtime;
use crate::cqrs::runtime::JoyDbBudgetRuntime;
use crate::models::{Currency, Money};
use calamine::{open_workbook, DataType, Reader, Xlsx};
use chrono::{DateTime, NaiveDate, Utc};
use dioxus::logger;
use dioxus::logger::tracing;
use uuid::Uuid;

pub fn import_from_skandia_excel(
    path: &str,
    user_id: &Uuid,
    budget_id: &Uuid,
    runtime: &JoyDbBudgetRuntime,
) -> anyhow::Result<u64> {
    let mut excel: Xlsx<_> = open_workbook(path)?;
    let mut imported = 0u64;
    let mut not_imported = 0u64;
    let mut total_rows= 0u64;
    if let Ok(r) = excel.worksheet_range("Kontoutdrag") {
        let mut account_number: Option<String> = None;

        for (row_num, row) in r.rows().enumerate() {
            if row_num == 0 {
                account_number = Some(row[1].to_string());
            } else if row_num > 4 {
                println!("row={:?}", row);
                let amount =
                    Money::new_cents((row[2].as_f64().unwrap() * 100.0) as i64, Currency::SEK);
                let balance =
                    Money::new_cents((row[3].as_f64().unwrap() * 100.0) as i64, Currency::SEK);
                let date_str = row[0].to_string();
                let naive_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?;

                // Convert to midnight UTC
                let date: DateTime<Utc> = naive_date
                    .and_hms_opt(0, 0, 0) // hours, minutes, seconds
                    .unwrap()
                    .and_utc();

                let description = row[1].to_string();
                let acct_no = if account_number.is_some() {
                    account_number.clone().unwrap()
                } else {
                    return Err(anyhow::anyhow!("Could not find account number"));
                };
                match runtime.add_transaction(
                    *budget_id,
                    &acct_no,
                    amount,
                    balance,
                    &description,
                    date,
                    *user_id,
                ) {
                    Ok(_) => {
                        imported += 1;
                        total_rows += 1;
                    }
                    Err(_) => {
                        not_imported += 1;
                        total_rows += 1;
                    }
                }
            }
        }
        tracing::info!("Imported {} transactions, skipped {} transactions, total {} transactions", imported, not_imported, total_rows);
    }

    if let Ok(Some(budget)) = runtime.load(budget_id) {
        let matches = budget.evaluate_rules();
        for (tx_id, item_id) in matches {
            let _ = runtime.connect_transaction(budget_id, &tx_id, &item_id, user_id);
        }
    }

    Ok(imported)
}

// pub fn import_bank_transactions(_bytes: Vec<u8>) -> anyhow::Result<()> {
// let mut csv_reader = csv::Reader::from_reader(bytes.as_slice());
// let mut _imported = 0;
// let mut _skipped = 0;
//
// /*
// Motsvarar kolumnerna i CSV filen direkt från Swedbank
//  */
// let csv_mapping: HashMap<&str, usize> = HashMap::from([
//     ("bookkeeping_date", 5),
//     ("transaction_date", 6),
//     ("currency_date", 7),
//     ("transaction_text", 9),
//     ("amount", 10),
//     ("account_total", 11),
//     ("reference", 8),
// ]);
// for r in csv_reader.records() {
//     let record = r?;
//     let bookkeeping_date = record.get(*csv_mapping.get("bookkeeping_date").unwrap()).unwrap();
//     let transaction_date = record.get(*csv_mapping.get("transaction_date").unwrap()).unwrap();
//     let currency_date = record.get(*csv_mapping.get("currency_date").unwrap()).unwrap();
//     let transaction_text = record.get(*csv_mapping.get("transaction_text").unwrap()).unwrap();
//     let amount = record.get(*csv_mapping.get("amount").unwrap()).unwrap();
//     let account_total = record.get(*csv_mapping.get("account_total").unwrap()).unwrap();
//     let reference = record.get(*csv_mapping.get("reference").unwrap()).unwrap();
//     let other_fields = format!(
//         "{}|{}|{}|{}|{}|{}|{}",
//         bookkeeping_date,
//         transaction_date,
//         currency_date,
//         transaction_text,
//         amount,
//         account_total,
//         reference,
//     );
//
//     // Calculate hash for the record
//     let bookkeeping_date = sea_orm::prelude::Date::from_str(bookkeeping_date)
//         .unwrap_or_else(|_| (sea_orm::prelude::Date::MIN));
//     let record_hash = calculate_bank_transaction_hash(&other_fields);
//
//     // Check if member with similar data already exists
//     if QueryCore::bank_transaction_exists_by_hash(conn, &record_hash).await {
//         _skipped += 1;
//         continue;
//     }
//     let mut amount = amount
//         .replace('−', "-") // U+2212 MINUS SIGN
//         .replace('–', "-") // EN DASH
//         .replace('—', "-") // EM DASH
//         .replace(',', ".") // Replace comma with dot
//         .split_whitespace()
//         .collect::<String>()
//         .parse::<Decimal>()
//         .unwrap_or_default();
//
//     // Create the member
//     let bank_transaction_model = bank_transaction::Model {
//         id: Uuid::default(),
//         bookkeeping_date,
//         transaction_text: transaction_text.to_string(),
//         reference: reference.to_string(),
//         amount,
//         other_fields: other_fields.to_string(),
//         hash: String::default(),
//     };
//
//     if let Err(e) =
//         MutationCore::create_bank_transaction(conn, bank_transaction_model)
//             .await
//     {
//         // Handle error (log it, but continue processing other records)
//         log::error!("Failed to create member: {}", e);
//     } else {
//         _imported += 1;
//     }
// }
//     Ok(())
// }
