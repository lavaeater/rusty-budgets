
use welds::errors::Result;

use welds::migrations::prelude::*;


pub(super) fn step(_state: &TableState) -> Result<MigrationStep> {
    let m = create_table("budgets")
        .id(|c| c("id", Type::Uuid))
        .column(|c| c("name", Type::String))
        .column(|c| c("default_budget", Type::Bool))
        .column(|c| c("created_at", Type::Datetime))
        .column(|c| c("updated_at", Type::Datetime))
        .column(|c| c("user_id", Type::Uuid).create_foreign_key("users", "id", OnDelete::Cascade));
    Ok(MigrationStep::new(
        "m20250710145549_create_table_budgets",
        m,
    ))
}
