use crate::errors::Result;
use welds::migrations::prelude::*;

pub async fn up(db: &dyn welds::TransactStart) -> Result<()> {
    let list: Vec<MigrationFn> = vec![
        m001_initial_schema,
        m002_transactions_table,
    ];
    welds::migrations::up(db, list.as_slice()).await?;
    Ok(())
}

/// Creates the four tables that back the joydb `AppState` models.
///
/// `budget_events` and `budgets` are intentionally schema-light: all domain
/// state lives in a `data` JSONB column so schema migrations aren't needed
/// as the domain model evolves.
#[allow(clippy::unnecessary_wraps)]
fn m001_initial_schema(_state: &TableState) -> Result<MigrationStep> {
    let steps = Steps::new()
        .add(
            create_table("budget_events")
                .id(|c| c("id", Type::Uuid))
                .column(|c| c("aggregate_id", Type::Uuid).create_index())
                .column(|c| c("timestamp", Type::IntBig))
                .column(|c| c("created_at", Type::DatetimeZone))
                .column(|c| c("user_id", Type::Uuid))
                .column(|c| c("data", Type::Json)),
        )
        .add(
            create_table("budgets")
                .id(|c| c("id", Type::Uuid))
                .column(|c| c("version", Type::IntBig))
                .column(|c| c("last_event", Type::IntBig))
                .column(|c| c("data", Type::Json)),
        )
        .add(
            create_table("users")
                .id(|c| c("id", Type::Uuid))
                .column(|c| c("user_name", Type::Text))
                .column(|c| c("email", Type::Text).create_unique_index())
                .column(|c| c("first_name", Type::Text))
                .column(|c| c("last_name", Type::Text))
                .column(|c| c("phone", Type::Text).is_null())
                .column(|c| c("birthday", Type::Date).is_null()),
        )
        .add(
            create_table("user_budgets")
                .id(|c| c("id", Type::Uuid))
                .column(|c| c("budgets", Type::Json)),
        );
    Ok(MigrationStep::new("m001_initial_schema", steps))
}

/// `BankTransaction` rows, previously embedded inline in every
/// `BudgetPeriod` inside the `budgets.data` blob — pulled out into their own
/// table so an aggregate load/snapshot no longer has to move the whole
/// transaction history on every request. See `PgRuntime::load`/`snapshot`
/// for how this table is kept in sync.
#[allow(clippy::unnecessary_wraps)]
fn m002_transactions_table(_state: &TableState) -> Result<MigrationStep> {
    let steps = Steps::new().add(
        create_table("transactions")
            .id(|c| c("id", Type::Uuid))
            .column(|c| c("budget_id", Type::Uuid).create_index())
            .column(|c| c("account_number", Type::Text))
            .column(|c| c("amount_cents", Type::IntBig))
            .column(|c| c("amount_currency", Type::Text))
            .column(|c| c("balance_cents", Type::IntBig))
            .column(|c| c("balance_currency", Type::Text))
            .column(|c| c("description", Type::Text))
            .column(|c| c("date", Type::DatetimeZone))
            .column(|c| c("actual_id", Type::Uuid).is_null())
            .column(|c| c("ignored", Type::Bool))
            .column(|c| c("tag_id", Type::Uuid).is_null()),
    );
    Ok(MigrationStep::new("m002_transactions_table", steps))
}
