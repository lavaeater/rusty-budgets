use welds::errors::Result;
#[cfg(feature = "server")]
use welds::migrations::prelude::*;

#[cfg(feature = "server")]
pub(super) fn step(_state: &TableState) -> Result<MigrationStep> {
    let m = create_table("budget_transactions")
        .id(|c| c("id", Type::Uuid))
        .column(|c| c("text", Type::String))
        .column(|c| {
            c("from_budget_item", Type::Uuid)
                .is_null()
                .create_foreign_key("budget_items", "id", OnDelete::Cascade)
        })
        .column(|c| {
            c("to_budget_item", Type::Uuid).create_foreign_key(
                "budget_items",
                "id",
                OnDelete::Cascade,
            )
        })
        .column(|c| c("amount", Type::Float))
        .column(|c| c("created_at", Type::Datetime))
        .column(|c| c("updated_at", Type::Datetime))
        .column(|c| {
            c("created_by", Type::Uuid).create_foreign_key("users", "id", OnDelete::Cascade)
        });
    Ok(MigrationStep::new(
        "m20250721110153_create_table_add_transactions",
        m,
    ))
}
