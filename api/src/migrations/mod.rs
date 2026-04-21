use crate::errors::Result;
use welds::migrations::prelude::*;

pub async fn up(db: &dyn welds::TransactStart) -> Result<()> {
    let list: Vec<MigrationFn> = vec![
        m001_initial_schema,
    ];
    welds::migrations::up(db, list.as_slice()).await?;
    Ok(())
}

/// Creates the four tables that back the joydb AppState models.
///
/// `budget_events` and `budgets` are intentionally schema-light: all domain
/// state lives in a `data` JSONB column so schema migrations aren't needed
/// as the domain model evolves.
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
