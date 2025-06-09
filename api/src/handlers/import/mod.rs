use std::collections::HashMap;
use crate::handlers::auth::login_required_middleware::login_required_middleware;
use crate::handlers::auth::required_role_middleware::RequiredRoleMiddleware;
use crate::{AppState, PaginationParams};
use entities::{bank_transaction, calculate_bank_transaction_hash, calculate_member_hash, member};
use poem::error::InternalServerError;
use poem::http::StatusCode;
use poem::web::{Data, Html, Multipart, Query};
use poem::{get, handler, post, EndpointExt, IntoResponse, Route};
use sea_orm::prelude::{Decimal, Uuid};
use service::{MutationCore, QueryCore};
use std::default::Default;
use std::str::FromStr;

#[handler]
pub async fn index(
    state: Data<&AppState>,
    Query(_params): Query<PaginationParams>,
) -> poem::Result<impl IntoResponse> {
    let mut ctx = tera::Context::new();

    let imports = QueryCore::list_imports(&state.conn)
        .await
        .map_err(InternalServerError)?;
    ctx.insert("imports", &imports);

    let body = state
        .templates
        .render("import/index.html.tera", &ctx)
        .map_err(InternalServerError)?;
    Ok(Html(body))
}

#[derive(Debug)]
enum ImportType {
    Members,
    Transactions,
}

#[handler]
pub async fn upload(
    state: Data<&AppState>,
    mut multipart: Multipart,
) -> poem::Result<impl IntoResponse> {
    let mut import_type = None;
    let mut file_data = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        if let Some(name) = field.name() {
            match name {
                "import_type" => {
                    if let Ok(value) = field.text().await {
                        import_type = Some(match value.as_str() {
                            "members" => ImportType::Members,
                            "transactions" => ImportType::Transactions,
                            _ => {
                                return Ok(
                                    StatusCode::ACCEPTED.with_header("HX-Redirect", "/import")
                                )
                            }
                        });
                    }
                }
                "file" => {
                    if let Ok(bytes) = field.bytes().await {
                        file_data = Some(bytes);
                    }
                }
                _ => {}
            }
        }
    }

    match (import_type, file_data) {
        (Some(import_type), Some(bytes)) => {
            match import_type {
                ImportType::Members => {
                    let mut csv_reader = csv::Reader::from_reader(bytes.as_slice());
                    let conn = &state.conn;
                    let mut _imported = 0;
                    let mut _skipped = 0;

                    for r in csv_reader.records() {
                        let record = r.map_err(InternalServerError)?;
                        let first_name = record.get(0).unwrap();
                        let last_name = record.get(1).unwrap();
                        let birthdate = record.get(2).unwrap();
                        let mobile_phone = record.get(3).unwrap();
                        let email = record.get(4).unwrap();

                        // Calculate hash for the record
                        let birth_date = sea_orm::prelude::Date::from_str(birthdate);

                        let birth_date_string = match birth_date {
                            Ok(date) => date.to_string(),
                            Err(_) => "".to_string(),
                        };

                        let birth_date = match birth_date {
                            Ok(date) => Some(date),
                            Err(_) => None,
                        };

                        let record_hash =
                            calculate_member_hash(first_name, last_name, &birth_date_string);

                        // Check if member with similar data already exists
                        if QueryCore::member_exists_by_hash(conn, &record_hash).await {
                            _skipped += 1;
                            continue;
                        }

                        let mobile_phone = if mobile_phone.is_empty() {
                            None
                        } else {
                            Some(mobile_phone.to_string())
                        };

                        let email = if email.is_empty() {
                            None
                        } else {
                            Some(email.to_string())
                        };

                        // Create the member
                        let member_model = member::Model {
                            id: Uuid::default(),
                            first_name: first_name.to_string(),
                            last_name: last_name.to_string(),
                            birth_date,
                            mobile_phone,
                            email,
                            hash: String::default(),
                        };

                        if let Err(e) = MutationCore::create_member(conn, member_model).await {
                            // Handle error (log it, but continue processing other records)
                            log::error!("Failed to create member: {}", e);
                        } else {
                            _imported += 1;
                        }
                    }
                    Ok(StatusCode::ACCEPTED.with_header("HX-Redirect", "/members"))
                }
                ImportType::Transactions => {
                    let mut csv_reader = csv::Reader::from_reader(bytes.as_slice());
                    let conn = &state.conn;
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
                    Ok(StatusCode::ACCEPTED.with_header("HX-Redirect", "/members"))
                }
            }
        }
        _ => Ok(StatusCode::ACCEPTED.with_header("HX-Redirect", "/import")),
    }
}

pub fn import_routes() -> Route {
    Route::new()
        .at("/", get(index).around(login_required_middleware))
        .at(
            "/upload",
            post(upload).with(RequiredRoleMiddleware::new("super_admin")),
        )
}
