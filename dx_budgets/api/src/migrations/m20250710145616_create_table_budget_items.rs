use welds::errors::Result;
#[cfg(feature = "server")]
use welds::migrations::prelude::*;
#[cfg(feature = "server")]
pub(super) fn step(_state: &TableState) -> Result<MigrationStep> {
    let m = create_table("budget_items")
        .id(|c| c("id", Type::Uuid))
        .column(|c| c("name", Type::String))
        .column(|c| c("amount", Type::Float))
        .column(|c| c("created_at", Type::Datetime))
        .column(|c| c("updated_at", Type::Datetime))
        .column(|c| c("created_by", Type::Uuid)
            .create_foreign_key("users", "id", OnDelete::Cascade))
        .column(|c| {
            c("budget_id", Type::Uuid)
                .create_foreign_key("budgets", "id", OnDelete::Cascade)
        });
    Ok(MigrationStep::new(
        "m20250710145616_create_table_budget_items",
        m,
    ))
}
