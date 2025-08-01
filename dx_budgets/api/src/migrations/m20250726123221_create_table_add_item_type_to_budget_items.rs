use welds::errors::Result;

use welds::migrations::prelude::*;


pub(super) fn step(state: &TableState) -> Result<MigrationStep> {
    let alter = change_table(state, "budget_items")?;
    let m = alter.add_column("item_type", Type::String);
    Ok(MigrationStep::new(
        "m20250726123221_create_table_add_item_type_to_budget_items",
        m,
    ))
}
