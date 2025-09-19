use alloc::vec::Vec;

pub fn import_bank_transactions(bytes: Vec<u8>) -> anyhow::Result<()> {

let mut csv_reader = csv::Reader::from_reader(bytes.as_slice());
let mut _imported = 0;
let mut _skipped = 0;

/*
Motsvarar kolumnerna i CSV filen direkt från Swedbank
 */
let csv_mapping: HashMap<&str, usize> = HashMap::from([
("bookkeeping_date", 5),
("transaction_date", 6),
("currency_date", 7),
("transaction_text", 9),
("amount",10),
("account_total", 11),
("reference",8),
]);
for r in csv_reader.records() {
    let record = r.map_err(InternalServerError)?;
    let bookkeeping_date = record.get(*csv_mapping.get("bookkeeping_date").unwrap()).unwrap();
    let transaction_date = record.get(*csv_mapping.get("transaction_date").unwrap()).unwrap();
    let currency_date = record.get(*csv_mapping.get("currency_date").unwrap()).unwrap();
    let transaction_text = record.get(*csv_mapping.get("transaction_text").unwrap()).unwrap();
    let amount = record.get(*csv_mapping.get("amount").unwrap()).unwrap();
    let account_total = record.get(*csv_mapping.get("account_total").unwrap()).unwrap();
    let reference = record.get(*csv_mapping.get("reference").unwrap()).unwrap();
    let other_fields = format!(
        "{}|{}|{}|{}|{}|{}|{}",
        bookkeeping_date,
        transaction_date,
        currency_date,
        transaction_text,
        amount,
        account_total,
        reference,
    );

    // Calculate hash for the record
    let bookkeeping_date = sea_orm::prelude::Date::from_str(bookkeeping_date)
        .unwrap_or_else(|_| (sea_orm::prelude::Date::MIN));
    let record_hash = calculate_bank_transaction_hash(&other_fields);

    // Check if member with similar data already exists
    if QueryCore::bank_transaction_exists_by_hash(conn, &record_hash).await {
        _skipped += 1;
        continue;
    }
    let mut amount = amount
        .replace('−', "-") // U+2212 MINUS SIGN
        .replace('–', "-") // EN DASH
        .replace('—', "-") // EM DASH
        .replace(',', ".") // Replace comma with dot
        .split_whitespace()
        .collect::<String>()
        .parse::<Decimal>()
        .unwrap_or_default();

    // Create the member
    let bank_transaction_model = bank_transaction::Model {
        id: Uuid::default(),
        bookkeeping_date,
        transaction_text: transaction_text.to_string(),
        reference: reference.to_string(),
        amount,
        other_fields: other_fields.to_string(),
        hash: String::default(),
    };

    if let Err(e) =
        MutationCore::create_bank_transaction(conn, bank_transaction_model)
            .await
    {
        // Handle error (log it, but continue processing other records)
        log::error!("Failed to create member: {}", e);
    } else {
        _imported += 1;
    }
}