//! Integration tests for `PgRuntime` against a real Postgres database (the
//! same one the app is configured to use via `DATABASE_URL` in the
//! workspace `.env`). Ignored by default so a plain `cargo test` stays
//! hermetic — run explicitly with:
//!
//! ```sh
//! cargo test -p api --features server --test pg_runtime -- --ignored
//! ```
//!
//! Every test uses a fresh random budget id and deletes everything it wrote
//! (`transactions`, `budget_events`, `budgets` rows) before returning, so
//! nothing is left behind in the real database.

#![cfg(feature = "server")]

use api::api_error::RustyError;
use api::cqrs::framework::{AsyncRuntime, DomainEvent};
use api::cqrs::runtime::{AsyncBudgetCommandsTrait, create_runtime};
use api::models::{Currency, Money, MonthBeginsOn};
use chrono::Utc;
use uuid::Uuid;

/// Deletes every row a test wrote for `budget_id`, across all three tables
/// touched by `PgRuntime`, regardless of how far the test got.
async fn cleanup(client: &dyn welds::Client, budget_id: Uuid) {
    let _ = api::pg_models::PgBankTransaction::where_col(|t| t.budget_id.equal(budget_id))
        .delete(client)
        .await;
    let _ = api::pg_models::PgStoredBudgetEvent::where_col(|e| e.aggregate_id.equal(budget_id))
        .delete(client)
        .await;
    let _ = api::pg_models::PgBudget::where_col(|b| b.id.equal(budget_id))
        .delete(client)
        .await;
}

#[tokio::test]
#[ignore = "requires a live Postgres database (DATABASE_URL)"]
async fn transaction_round_trips_through_the_relational_table() -> Result<(), RustyError> {
    let rt = create_runtime().await;
    let user_id = Uuid::new_v4();

    let budget_id = rt
        .create_budget(user_id, "PgRuntime test budget", false, MonthBeginsOn::default(), Currency::SEK)
        .await?;

    let tx_id = rt
        .add_transaction(
            user_id,
            budget_id,
            "1234",
            Money::new_cents(-5000, Currency::SEK),
            Money::new_cents(100_000, Currency::SEK),
            "Test transaction",
            Utc::now(),
        )
        .await?;

    // Force a real snapshot: this is what moves the transaction out of the
    // append-only event log and into the `transactions` table.
    let budget = rt.load(budget_id).await?;
    rt.snapshot(&budget).await?;

    // Reload from scratch — nothing but `budgets.data` + `transactions` +
    // any trailing events feeds this, so this proves the round trip.
    let reloaded = rt.load(budget_id).await?;
    let tx = reloaded.get_transaction(tx_id);
    assert!(tx.is_some(), "transaction must survive a snapshot + reload");
    let tx = tx.unwrap();
    assert_eq!(tx.description, "Test transaction");
    assert_eq!(tx.amount, Money::new_cents(-5000, Currency::SEK));

    cleanup(rt.client(), budget_id).await;
    Ok(())
}

#[tokio::test]
#[ignore = "requires a live Postgres database (DATABASE_URL)"]
async fn tagging_a_transaction_persists_through_the_relational_table() -> Result<(), RustyError> {
    let rt = create_runtime().await;
    let user_id = Uuid::new_v4();

    let budget_id = rt
        .create_budget(user_id, "PgRuntime test budget", false, MonthBeginsOn::default(), Currency::SEK)
        .await?;
    let tag_id = rt
        .create_tag(user_id, budget_id, "Groceries".to_string(), api::models::Periodicity::Monthly)
        .await?;
    let tx_id = rt
        .add_transaction(
            user_id,
            budget_id,
            "1234",
            Money::new_cents(-1234, Currency::SEK),
            Money::new_cents(100_000, Currency::SEK),
            "Ica",
            Utc::now(),
        )
        .await?;

    // Snapshot once so the transaction is in the relational table, then tag
    // it (a trailing event on top of that snapshot) and snapshot again.
    let budget = rt.load(budget_id).await?;
    rt.snapshot(&budget).await?;
    let mut budget = rt.load(budget_id).await?;
    let ev = budget.do_transaction_tagged(tx_id, tag_id)?;
    ev.apply(&mut budget);
    rt.append(user_id, ev.into()).await?;
    rt.snapshot(&budget).await?;

    let reloaded = rt.load(budget_id).await?;
    let tx = reloaded.get_transaction(tx_id).expect("transaction must still exist");
    assert_eq!(tx.tag_id, Some(tag_id), "tag must survive snapshot + reload");

    cleanup(rt.client(), budget_id).await;
    Ok(())
}

#[tokio::test]
#[ignore = "requires a live Postgres database (DATABASE_URL)"]
async fn snapshot_keeps_the_budgets_blob_small() -> Result<(), RustyError> {
    let rt = create_runtime().await;
    let user_id = Uuid::new_v4();

    let budget_id = rt
        .create_budget(user_id, "PgRuntime test budget", false, MonthBeginsOn::default(), Currency::SEK)
        .await?;
    for i in 0..25 {
        rt.add_transaction(
            user_id,
            budget_id,
            "1234",
            Money::new_cents(-100 * i, Currency::SEK),
            Money::new_cents(100_000, Currency::SEK),
            &format!("Transaction {i}"),
            Utc::now(),
        )
        .await?;
    }

    let budget = rt.load(budget_id).await?;
    rt.snapshot(&budget).await?;

    let row = api::pg_models::PgBudget::find_by_id(rt.client(), budget_id)
        .await?
        .expect("budget row must exist");
    let blob_bytes = row.data.to_string().len();
    // 25 transactions embedded inline would be several KB of JSON on their
    // own; a slim blob (no transactions, no hashes) should stay well under
    // that regardless of how many transactions exist.
    assert!(
        blob_bytes < 2_000,
        "budgets.data should stay small once transactions move to their own table, was {blob_bytes} bytes"
    );

    let tx_rows = api::pg_models::PgBankTransaction::where_col(|t| t.budget_id.equal(budget_id))
        .run(rt.client())
        .await?;
    assert_eq!(tx_rows.len(), 25, "all transactions must land in the relational table");

    cleanup(rt.client(), budget_id).await;
    Ok(())
}

#[tokio::test]
#[ignore = "one-off: verifies full budget export/import against the production budget"]
async fn full_budget_export_import_round_trips_production_data() -> Result<(), RustyError> {
    let rt = create_runtime().await;
    let source_budget_id = Uuid::parse_str("73d06c2b-8be8-43f0-a884-ae71de735657").unwrap();
    let fake_user_id = Uuid::new_v4();

    let json = api::db::export_budget(source_budget_id).await?;
    let new_budget_id = api::db::import_budget(fake_user_id, &json).await?;

    let source = rt.load(source_budget_id).await?;
    let imported = rt.load(new_budget_id).await?;

    let source_tx_count: usize = source.periods.iter().map(|p| p.transactions.len()).sum();
    let imported_tx_count: usize = imported.periods.iter().map(|p| p.transactions.len()).sum();
    println!(
        "source: {} periods, {} transactions, {} tags, {} rules, {} transfer rules",
        source.periods.len(), source_tx_count, source.tags.len(), source.match_rules.len(), source.transfer_rules.len()
    );
    println!(
        "imported: {} periods, {} transactions, {} tags, {} rules, {} transfer rules",
        imported.periods.len(), imported_tx_count, imported.tags.len(), imported.match_rules.len(), imported.transfer_rules.len()
    );
    assert_eq!(imported_tx_count, source_tx_count);
    assert_eq!(imported.periods.len(), source.periods.len());
    assert_eq!(imported.tags.len(), source.tags.len());
    assert_eq!(imported.match_rules.len(), source.match_rules.len());
    assert_eq!(imported.transfer_rules.len(), source.transfer_rules.len());
    assert_ne!(imported.id, source.id);
    assert_eq!(imported.user_id, fake_user_id);
    assert!(!imported.default_budget);

    // Transaction ids must never collide with the source's — that's the
    // actual bug this whole test exists to catch.
    let source_ids: std::collections::HashSet<Uuid> =
        source.periods.iter().flat_map(|p| p.transactions.iter().map(|t| t.id)).collect();
    let imported_ids: std::collections::HashSet<Uuid> =
        imported.periods.iter().flat_map(|p| p.transactions.iter().map(|t| t.id)).collect();
    assert!(
        source_ids.is_disjoint(&imported_ids),
        "imported transaction ids must never collide with the source budget's"
    );

    cleanup(rt.client(), new_budget_id).await;
    let _ = api::pg_models::PgUserBudgets::where_col(|b| b.id.equal(fake_user_id))
        .delete(rt.client())
        .await;
    Ok(())
}

