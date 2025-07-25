#[cfg(feature = "server")]
use welds::errors::Result;
#[cfg(feature = "server")]
use welds::migrations::prelude::*;

#[cfg(feature = "server")]
pub(super) fn step(_state: &TableState) -> Result<MigrationStep> {
    let m = create_table("budgets")
        .id(|c| c("id", Type::Uuid))
        .column(|c| c("name", Type::String))
        .column(|c| c("default_budget", Type::Bool))
        .column(|c| c("user_id", Type::Uuid).create_foreign_key("users", "id", OnDelete::Cascade));
    Ok(MigrationStep::new(
        "m20250710145549_create_table_budgets",
        m,
    ))
}
