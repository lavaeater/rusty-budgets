use welds::errors::Result;
use welds::migrations::prelude::*;

pub(super) fn step(state: &TableState) -> Result<MigrationStep> {
    let m = change_table(state, "budget_items")
        .id(|c| c("id", Type::Uuid))
        .column(|c| c("item_type", Type::String));
    Ok(MigrationStep::new(
        "m20250726123221_create_table_add_budget_item_type_to_budget_items",
        m,
    ))
}
